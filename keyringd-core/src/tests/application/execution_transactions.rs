use super::fixture::*;
use crate::model::ssh_trust::{SshCertificateAuthorityTrustEntry, SshDeviceKeyTrustEntry};
use crate::service::DeviceKeyProtectionChange;
use crate::test_support::TestOperation;

keyringd_testkit::scenario! {
    group "KeyringdApplication::setup_profile" {
        case "Attempts to clean up a provisioned Device Key when profile persistence fails" {
            why "A failed commit must not publish a partial profile or leave an unreferenced private key behind."
            async: tokio;
            timeout: std::time::Duration::from_secs(10);

            given "an empty application whose profile store rejects saves" {
                let store = Arc::new(MemoryRepository::default());
                store.set_save_failure(true);
                let device_keys = TrackingDeviceKeyService::default();
                let application = KeyringdApplication::new(
                    device_keys.clone(),
                    TestPasskeyService::new(),
                    store.clone(),

                )
                .unwrap();
            }
            when "profile setup provisions a Device Key and then fails to persist the profile" {
                let result = application
                    .setup_profile(
                        &Default::default(),
                        &Default::default(),
                        ProfileSetupRequest::new("Default".to_owned(), PasskeyChoice::CreateNew),
                        |_| {},
                        crate::test_support::reject_credential_selection,
                    )
                    .await;
            }
            then "setup reports the profile-store failure without publishing a profile" {
                assert!(matches!(result, Err(crate::application::ProfileSetupFailure::Failed(ApplicationError::ProfileRepository(_)))));
                assert!(application.profiles().is_empty());
                assert!(store.snapshot().unwrap().profiles.is_empty());
            }
            then "deletion is attempted for the generated Device Key as compensation" {
                let generated_handles = device_keys.generated_handles();
                let deleted_handles = device_keys.deleted_handles();
                assert_eq!(generated_handles.len(), 1);
                assert_eq!(deleted_handles.len(), 1);
                assert_eq!(deleted_handles[0].as_ref(), generated_handles[0].as_ref());
            }
        }
    }
}

keyringd_testkit::scenario! {
    group "KeyringdApplication::setup_profile" {
    case "Attempts to clean up a newly generated Device Key when certificate issuance fails" {
        why "A Device Key without its certificate cannot be published and should not be left behind after certificate issuance fails."
        async: tokio;
        timeout: std::time::Duration::from_secs(10);

        given "an empty application whose FIDO service fails certificate issuance" {
            let store = Arc::new(MemoryRepository::default());
            let device_keys = TrackingDeviceKeyService::default();
            let application = KeyringdApplication::new(
                device_keys.clone(),
                failing_assertion_fido(),
                store,

            )
            .unwrap();
        }
        when "profile setup generates a Device Key and attempts to issue its certificate" {
            let result = application
                .setup_profile(
                    &Default::default(),
                    &Default::default(),
                    ProfileSetupRequest::new("Default".to_owned(), PasskeyChoice::CreateNew),
                    |_| {},
                    crate::test_support::reject_credential_selection,
                )
                .await;

        }
        then "setup returns ProfileSetupFailure::PasskeyNotFound" {
            assert!(matches!(
                result,
                Err(crate::application::ProfileSetupFailure::PasskeyNotFound)
            ));
        }
        then "deletion is attempted for the newly generated Device Key as compensation" {
            let generated_handles = device_keys.generated_handles();
            let deleted_handles = device_keys.deleted_handles();
            assert_eq!(generated_handles.len(), 1);
            assert_eq!(deleted_handles.len(), 1);
            assert_eq!(deleted_handles[0].as_ref(), generated_handles[0].as_ref());
        }
        then "no incomplete profile is published" {
            assert!(application.profiles().is_empty());
        }

    }
    }
}

keyringd_testkit::scenario! {
    group "KeyringdApplication::change_device_key_protection" {
        case "Keeps the original device-key protection when persistence fails" {
            async: tokio;
            timeout: std::time::Duration::from_secs(10);

            given "a completed profile with its original Local authentication" {
                let (application, store, device_keys, id) = completed_application().await;
                let original = application.profiles()[0].clone();
                let original_handle = store.snapshot().unwrap().profiles[0]
                    .device_key().handle()
                    .clone();
                let original_device_trust_entry = SshDeviceKeyTrustEntry::from_profile(&original);
            }
            given "the profile store will reject the next save" {
                store.set_save_failure(true);
            }
            when "device-key user verification is enabled" {
                let result = application
                    .change_device_key_protection(&Default::default(), &id, DeviceKeyProtectionChange::Enable)
                    .await;

            }
            then "the protection change returns a profile-store error" {
                assert!(matches!(result, Err(ApplicationError::ProfileRepository(_))));
            }
            then "the runtime profile retains its original key and protection setting" {
                let current = &application.profiles()[0];
                assert_eq!(SshDeviceKeyTrustEntry::from_profile(current), original_device_trust_entry);
                assert_eq!(
                    current.device_key().local_authentication(),
                    original.device_key().local_authentication()
                );
            }
            then "no additional Device Key is generated and the existing key is not deleted" {
                assert_eq!(device_keys.generated_count(), 1);
                assert!(device_keys.deleted_handles().is_empty());
            }
            then "the persisted profile retains the original Device Key handle" {
                let persisted = store.snapshot().unwrap();
                assert_eq!(persisted.profiles[0].device_key().handle().as_ref(), original_handle.as_ref());
            }

        }
    }
}

keyringd_testkit::scenario! {
    group "KeyringdApplication::replace_device_key" {
        case "Commits the replacement device key before deleting the old key" {
            async: tokio;
            timeout: std::time::Duration::from_secs(10);

            given "a completed profile and a journal shared by profile persistence and device-key deletion" {
                let (application, store, device_keys, id, journal) = completed_application_with_journal().await;
                let old_handle = store.snapshot().unwrap().profiles[0].device_key().handle().clone();
                let ca_trust_entry = SshCertificateAuthorityTrustEntry::from_profile(&application.profiles()[0]);
                let old_device_trust_entry = SshDeviceKeyTrustEntry::from_profile(&application.profiles()[0]);

            }
            when "the profile device key is replaced" {
                application.replace_device_key(&Default::default(), &id).await.unwrap();

            }
            then "the runtime and persisted profile use the replacement Device Key" {
                let profile = &application.profiles()[0];
                let persisted = store.snapshot().unwrap();
                let generated_handles = device_keys.generated_handles();
                assert_eq!(persisted.profiles.len(), 1);
                assert_eq!(generated_handles.len(), 2);
                assert_eq!(
                    persisted.profiles[0].device_key().handle().as_ref(),
                    generated_handles[1].as_ref()
                );
                assert_ne!(SshDeviceKeyTrustEntry::from_profile(profile), old_device_trust_entry);
            }
            then "the CA trust entry remains stable across Device Key replacement" {
                assert_eq!(
                    SshCertificateAuthorityTrustEntry::from_profile(profile),
                    ca_trust_entry
                );
            }
            then "the old Device Key is deleted after the replacement is generated" {
                assert_eq!(device_keys.generated_count(), 2);
                assert_eq!(device_keys.deleted_handles().len(), 1);
                assert_eq!(
                    device_keys.deleted_handles()[0].as_ref(),
                    old_handle.as_ref()
                );
            }
            then "profile persistence is recorded before deletion of the old Device Key" {
                assert_eq!(
                    journal.operations(),
                    [
                        TestOperation::ProfilesSaved(vec![b"tracking-key-2".to_vec()]),
                        TestOperation::DeviceKeyDeleted(old_handle.as_ref().to_vec()),
                    ]
                );
            }

        }
        case "Preserves the referenced keys when replacement persistence fails" {
            why "A failed profile commit must not publish the replacement or delete the still-referenced old key; the now-unreferenced replacement should be cleaned up on a best-effort basis."
            async: tokio;
            timeout: std::time::Duration::from_secs(10);

            given "a completed profile whose profile store rejects subsequent saves" {
                let (application, store, device_keys, id) = completed_application().await;
                let old_handle = store.snapshot().unwrap().profiles[0].device_key().handle().clone();
                let old_device_trust_entry = SshDeviceKeyTrustEntry::from_profile(&application.profiles()[0]);
                store.set_save_failure(true);

            }
            when "the profile device key is replaced" {
                let result = application.replace_device_key(&Default::default(), &id).await;
            }
            then "replacement returns a profile-store error" {
                assert!(matches!(result, Err(ApplicationError::ProfileRepository(_))));
            }
            then "the runtime profile continues using the old Device Key" {
                assert_eq!(
                    SshDeviceKeyTrustEntry::from_profile(&application.profiles()[0]),
                    old_device_trust_entry
                );
            }
            then "deletion is attempted for the newly generated replacement, not the old Device Key" {
                let generated_handles = device_keys.generated_handles();
                assert_eq!(generated_handles.len(), 2);
                let replacement_handle = generated_handles.last().unwrap();
                assert_ne!(replacement_handle.as_ref(), old_handle.as_ref());
                let deleted_handles = device_keys.deleted_handles();
                assert_eq!(deleted_handles.len(), 1);
                assert_eq!(deleted_handles[0].as_ref(), replacement_handle.as_ref());
            }
            then "the persisted profile continues referring to the old Device Key" {
                assert_eq!(
                    store.snapshot().unwrap().profiles[0].device_key().handle().as_ref(),
                    old_handle.as_ref()
                );
            }
            when "a retry replaces the document but cannot sync its directory" {
                store.set_save_failure(false);
                store.fail_directory_sync();
                let result = application.replace_device_key(&Default::default(), &id).await;
            }
            then "the visible document uses the replacement and neither possible key is deleted" {
                assert!(matches!(result, Err(ApplicationError::ProfileDurabilityUncertain(_))));
                assert_eq!(device_keys.deleted_handles().len(), 1);
                assert_eq!(application.profiles()[0].device_key().handle().as_ref(), device_keys.generated_handles().last().unwrap().as_ref());
                assert_eq!(store.snapshot().unwrap().profiles[0].device_key().handle().as_ref(), application.profiles()[0].device_key().handle().as_ref());
            }
        }
        case "Keeps the old key and attempts to clean up the replacement when the FIDO operation is canceled" {
            why "Canceled certificate issuance must leave the current profile intact while cleaning up the unreferenced replacement on a best-effort basis."
            async: tokio;
            timeout: std::time::Duration::from_secs(10);

            given "a completed profile and a FIDO service that cancels certificate issuance" {
                let (store, id) = seed_default_profile(b"old-key").await;
                let device_keys = TrackingDeviceKeyService::default();
                let application = KeyringdApplication::new(
                    device_keys.clone(),
                    crate::test_support::cancelled_fido(),
                    store.clone(),

                )
                .unwrap();
                let original = store.snapshot().unwrap().profiles[0].device_key().handle().clone();
                let original_device_trust_entry = SshDeviceKeyTrustEntry::from_profile(&application.profiles()[0]);
            }
            when "the profile device key is replaced" {
                let result = application.replace_device_key(&Default::default(), &id).await;
            }
            then "replacement returns OperationCanceled" {
                assert!(matches!(result, Err(ApplicationError::OperationCanceled)));
            }
            then "the runtime and persisted profile continue using the old Device Key" {
                assert_eq!(
                    SshDeviceKeyTrustEntry::from_profile(&application.profiles()[0]),
                    original_device_trust_entry
                );
                assert_eq!(
                    store.snapshot().unwrap().profiles[0].device_key().handle().as_ref(),
                    original.as_ref()
                );
            }
            then "deletion is attempted only for the unused replacement Device Key" {
                let generated_handles = device_keys.generated_handles();
                let deleted_handles = device_keys.deleted_handles();
                assert_eq!(generated_handles.len(), 1);
                assert_eq!(deleted_handles.len(), 1);
                assert_eq!(deleted_handles[0].as_ref(), generated_handles[0].as_ref());
                assert_ne!(deleted_handles[0].as_ref(), original.as_ref());
            }
        }
    }
}

keyringd_testkit::scenario! {
    group "KeyringdApplication::remove_profile" {
        case "Keeps the profile and device key when removal cannot be persisted" {
            async: tokio;
            timeout: std::time::Duration::from_secs(10);

            given "a completed profile whose profile store rejects subsequent saves" {
                let (application, store, device_keys, id) = completed_application().await;
                store.set_save_failure(true);

            }
            when "the profile is removed" {
                let result = application.remove_profile(&Default::default(), &id).await;

            }
            then "removal returns a profile-store error" {
                assert!(matches!(result, Err(ApplicationError::ProfileRepository(_))));
            }
            then "the profile remains available at runtime" {
                assert_eq!(application.profiles().len(), 1);
                assert_eq!(application.profiles()[0].id(), &id);
            }
            then "the profile remains in persistent storage" {
                assert_eq!(store.snapshot().unwrap().profiles[0].id().clone(), id);
            }
            then "the profile Device Key is not deleted" {
                assert!(device_keys.deleted_handles().is_empty());
            }

        }
        case "Persists profile removal before attempting to delete its Device Key" {
            why "The Device Key must remain usable until profile removal is durable, while failure to clean up an already-unreferenced key must not restore the profile."
            async: tokio;
            timeout: std::time::Duration::from_secs(10);

            given "a completed profile and a journal shared by profile persistence and device-key deletion" {
                let (application, store, device_keys, id, journal) = completed_application_with_journal().await;
                let handle = store.snapshot().unwrap().profiles[0].device_key().handle().clone();

            }
            when "the profile is removed" {
                application.remove_profile(&Default::default(), &id).await.unwrap();

            }
            then "the profile is removed from runtime and persistent storage" {
                assert!(application.profiles().is_empty());
                assert!(store.snapshot().unwrap().profiles.is_empty());
            }
            then "deletion is attempted for the removed profile's Device Key" {
                assert_eq!(device_keys.deleted_handles().len(), 1);
                assert_eq!(device_keys.deleted_handles()[0].as_ref(), handle.as_ref());
            }
            then "the empty profile set is persisted before Device Key deletion is attempted" {
                assert_eq!(
                    journal.operations(),
                    [
                        TestOperation::ProfilesSaved(Vec::new()),
                        TestOperation::DeviceKeyDeleted(handle.as_ref().to_vec()),
                    ]
                );
            }

        }
    }
}

keyringd_testkit::scenario! {
    group "KeyringdApplication::rename_profile" {
    case "Leaves runtime and persisted names unchanged when persistence fails" {
        async: tokio;
        timeout: std::time::Duration::from_secs(10);

        given "a completed profile whose profile store rejects subsequent saves" {
            let (application, store, _device_keys, id) = completed_application().await;
            let before = application.profiles()[0].display_name().to_owned();
            store.set_save_failure(true);

        }
        when "the profile is renamed" {
            let result = application.rename_profile(&id, "Renamed".to_string());

        }
        then "rename returns a profile-store error" {
            assert!(matches!(result, Err(ApplicationError::ProfileRepository(_))));
        }
        then "the runtime profile retains its original display name" {
            assert_eq!(application.profiles()[0].display_name(), before);
        }
        then "the persisted profile retains its original display name" {
            assert_eq!(store.snapshot().unwrap().profiles[0].display_name(), before);
        }

    }
    }
}

keyringd_testkit::scenario! {
    group "KeyringdApplication::set_profile_enabled" {
    case "Does not publish an enablement change that cannot be persisted" {
        async: tokio;
        timeout: std::time::Duration::from_secs(10);

        given "a completed profile whose profile store rejects subsequent saves" {
            let (application, store, _device_keys, id) = completed_application().await;
            let before = application.profiles()[0].is_enabled();
            store.set_save_failure(true);

        }
        when "the profile enablement is toggled" {
            let result = application.set_profile_enabled(&id, !before);

        }
        then "the enablement change returns a profile-store error" {
            assert!(matches!(result, Err(ApplicationError::ProfileRepository(_))));
        }
        then "the runtime profile retains its original enabled state" {
            assert_eq!(application.profiles()[0].is_enabled(), before);
        }
    }
    }
}

keyringd_testkit::scenario! {
    group "KeyringdApplication::set_profile_order" {
        case "Leaves runtime and persisted profile order unchanged when persistence fails" {
            async: tokio;
            timeout: std::time::Duration::from_secs(10);

            given "two completed profiles whose profile store rejects subsequent saves" {
                let (application, store, first_id, second_id) = application_with_two_profiles().await;
                store.set_save_failure(true);

            }
            when "the profiles are reordered" {
                let result = application.set_profile_order(&[second_id.clone(), first_id.clone()]);

            }
            then "reordering returns a profile-store error" {
                assert!(matches!(result, Err(ApplicationError::ProfileRepository(_))));
            }
            then "runtime profiles retain their original order" {
                assert_eq!(application.profiles()[0].id(), &first_id);
                assert_eq!(application.profiles()[1].id(), &second_id);
            }
            then "persisted profiles retain their original order" {
                let persisted = store.snapshot().unwrap().profiles;
                assert_eq!(persisted[0].id().clone(), first_id);
                assert_eq!(persisted[1].id().clone(), second_id);
            }

        }
    }
}
