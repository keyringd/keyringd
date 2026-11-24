use super::fixture::*;
use crate::application::OperationRejectionReason;
use crate::model::ssh_trust::{SshCertificateAuthorityTrustEntry, SshDeviceKeyTrustEntry};
use crate::service::DeviceKeyProtectionChange;

keyringd_testkit::scenario! {
    group "KeyringdApplication::rename_profile" {
        case "Invalid profile names return a typed reason without changing state" {
            async: tokio;
            timeout: std::time::Duration::from_secs(10);
            verify "invalid profile names return a typed reason without changing state" {
                let (store, id) = seed_default_profile(b"existing-key").await;
                let application = KeyringdApplication::new(
                    crate::test_support::successful_device_keys(),
                    TestPasskeyService::new(),
                    store.clone(),
                )
                .unwrap();

                for name in ["   ".to_owned(), "é".repeat(81)] {
                    let error = application.rename_profile(&id, name).unwrap_err();
                    assert!(matches!(
                        error,
                        ApplicationError::OperationRejected(OperationRejectionReason::InvalidProfileName)
                    ));
                    assert_eq!(
                        error.user_message(),
                        "Enter a profile name with 1 to 80 characters, without surrounding whitespace, line breaks, or control characters."
                    );
                    assert_eq!(application.profiles()[0].display_name(), "Default");
                    assert_eq!(
                        store.snapshot().unwrap().profiles[0].display_name(),
                        "Default"
                    );
                }
            }
        }
    }
}

keyringd_testkit::scenario! {
    group "KeyringdApplication::rename_profile" {
    case "Persists a profile rename and updates the runtime profile" {
        async: tokio;
        timeout: std::time::Duration::from_secs(10);

        given "a completed profile named Default" {
            let (persistence, id) = seed_default_profile(b"existing-key").await;
            let application = KeyringdApplication::new(
                crate::test_support::successful_device_keys(),
                TestPasskeyService::new(),
                persistence.clone(),

            )
            .unwrap();

        }
        when "the profile is renamed to Production with surrounding whitespace" {
            application
                .rename_profile(&id, " Production ".to_string())
                .unwrap();

        }
        then "both the runtime repository and persisted record use the new name" {
            assert_eq!(application.profiles()[0].display_name(), "Production");
            assert_eq!(
                persistence.snapshot().unwrap().profiles[0].display_name(),
                "Production"
            );
        }

    }
    }
}

keyringd_testkit::scenario! {
    group "KeyringdApplication::change_device_key_protection" {
        case "Reuses the existing certificate when changing protection preserves the public key" {
            async: tokio;
            timeout: std::time::Duration::from_secs(10);

            given "a persisted profile, a protection backend that preserves its public key, and a FIDO service that rejects certificate issuance" {
                let (store, id) = seed_default_profile(b"old-key").await;
                let device_keys = TrackingDeviceKeyService::default();
                let application = KeyringdApplication::new(
                    device_keys.clone(),
                    failing_assertion_fido(),
                    store.clone(),

                )
                .unwrap();
                let ca_before = SshCertificateAuthorityTrustEntry::from_profile(&application.profiles()[0]);
                let device_before = SshDeviceKeyTrustEntry::from_profile(&application.profiles()[0]);
                let certificate_before = application.profiles()[0].device_key().certificate().to_key_blob();
            }
            when "device-key user verification is enabled" {
                let result = application
                    .change_device_key_protection(&Default::default(), &id, DeviceKeyProtectionChange::Enable)
                    .await;
            }
            then "the protection change succeeds without invoking certificate issuance" {
                result.unwrap();
                assert_eq!(
                    application.profiles()[0].device_key().local_authentication(),
                    LocalAuthentication::Required
                );
            }
            then "the persisted Device Key handle changes without changing the public keys or ECDSA registration entries" {
                let profile = &application.profiles()[0];
                assert_eq!(profile.device_key().certificate().to_key_blob(), certificate_before);
                assert_eq!(store.snapshot().unwrap().profiles[0].device_key().certificate().to_key_blob(), certificate_before);
                assert!(
                    store
                        .snapshot()
                        .unwrap()
                        .profiles[0]
                        .device_key().handle()
                        .as_ref()
                        .ends_with(b"-updated-protection")
                );
                assert_eq!(SshCertificateAuthorityTrustEntry::from_profile(profile), ca_before);
                assert_eq!(SshDeviceKeyTrustEntry::from_profile(profile), device_before);
            }
            then "no new key pair is generated and no key is deleted" {
                assert_eq!(device_keys.generated_count(), 0);
                assert!(device_keys.deleted_handles().is_empty());
            }
        }
    }
}

keyringd_testkit::scenario! {
    group "KeyringdApplication::set_profile_order" {
        case "Persists a new profile order" {
            async: tokio;
            timeout: std::time::Duration::from_secs(10);

            given "two completed profiles in their original order" {
                let (application, store, first_id, second_id) = application_with_two_profiles().await;
            }
            when "the profiles are reordered in reverse" {
                let changed = application
                    .set_profile_order(&[second_id.clone(), first_id.clone()])
                    .unwrap();
            }
            then "the operation reports that the order changed" {
                assert!(changed);
            }
            then "runtime profiles use the requested order" {
                assert_eq!(application.profiles()[0].id(), &second_id);
                assert_eq!(application.profiles()[1].id(), &first_id);
            }
            then "persisted profiles use the requested order" {
                let persisted = store.snapshot().unwrap().profiles;
                assert_eq!(persisted[0].id().clone(), second_id);
                assert_eq!(persisted[1].id().clone(), first_id);
            }
        }

        case "Rejects incomplete, duplicate, and unknown profile orders without changing state" {
            async: tokio;
            timeout: std::time::Duration::from_secs(10);

            given "two completed profiles in their original order" {
                let (application, store, first_id, second_id) = application_with_two_profiles().await;
                let unknown_id = ProfileId::from_stored("unknown".to_owned());
            }
            when "orders with a missing profile, a duplicate profile, and an unknown profile are requested" {
                let missing = application.set_profile_order(std::slice::from_ref(&first_id));
                let duplicate = application.set_profile_order(&[first_id.clone(), first_id.clone()]);
                let unknown = application.set_profile_order(&[first_id.clone(), unknown_id]);
            }
            then "each invalid order request returns OperationRejected with its specific reason" {
                for (result, expected) in [
                    (missing, OperationRejectionReason::ProfileOrderLength),
                    (duplicate, OperationRejectionReason::DuplicateProfileInOrder),
                    (unknown, OperationRejectionReason::UnknownProfileInOrder),
                ] {
                    assert!(matches!(result, Err(ApplicationError::OperationRejected(reason)) if reason == expected));
                }
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

        case "Treats the current profile order as a no-op without persisting" {
            async: tokio;
            timeout: std::time::Duration::from_secs(10);

            given "a completed profile and an empty operation journal" {
                let (application, _store, _device_keys, id, journal) = completed_application_with_journal().await;
            }
            when "the current profile order is requested again" {
                let changed = application.set_profile_order(std::slice::from_ref(&id)).unwrap();
            }
            then "the operation reports that no order change occurred" {
                assert!(!changed);
            }
            then "no persistence write is performed" {
                assert!(journal.operations().is_empty());
            }
        }
    }
}
