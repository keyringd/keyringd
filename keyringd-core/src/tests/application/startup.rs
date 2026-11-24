use super::fixture::*;

keyringd_testkit::scenario! {
    group "KeyringdApplication::new" {
    case "Creates an empty application from an empty profile store" {
        async: tokio;
        timeout: std::time::Duration::from_secs(10);

        given "an empty in-memory profile store" {
            let persistence = Arc::new(MemoryRepository::default());

        }
        when "the application is initialized from the store" {
            let agent = KeyringdApplication::new(
                crate::test_support::successful_device_keys(),
                TestPasskeyService::new(),
                persistence.clone(),

            )
            .unwrap();

        }
        then "it contains no completed profiles and leaves the store empty" {
            assert!(agent.profiles().is_empty());
            assert!(persistence.snapshot().unwrap().profiles.is_empty());
        }

    }

    case "Restores a persisted profile without accessing the Device Key service" {
        async: tokio;
        timeout: std::time::Duration::from_secs(10);

        given "a persisted profile with an unavailable device-key handle" {
            let (persistence, id) = seed_default_profile(b"missing-key").await;
            // Any unexpected service call fails because no expectations are configured.
            let device_keys = MockDeviceKeyService::new();
        }
        when "the application is initialized" {
            let agent = KeyringdApplication::new(
                device_keys,
                TestPasskeyService::new(),
                persistence.clone(),

            )
            .unwrap();
        }
        then "it publishes the saved profile and preserves the stored handle without checking the key" {
            assert_eq!(agent.profiles().len(), 1);
            assert_eq!(agent.profiles()[0].id(), &id);
            assert_eq!(agent.profiles()[0].device_key().handle().as_ref(), b"missing-key");
            assert_eq!(
                persistence.snapshot().unwrap().profiles[0].device_key().handle().as_ref(),
                b"missing-key"
            );
        }
    }

    case "Preserves persisted profile order while loading every profile" {
        async: tokio;
        timeout: std::time::Duration::from_secs(10);

        given "two persisted profiles ordered Personal then Work" {
            let mut profiles = Vec::new();
            for name in ["Personal", "Work"] {
                profiles.push(
                    persisted_profile(
                        ProfileId::from_stored(name.to_ascii_lowercase()),
                        name,
                        DevicePrivateKeyHandle::new(format!("{name}-key").into_bytes()),
                    )
                    .await,
                );
            }
            let state = ProfilesDocument {
                schema_version: PROFILES_SCHEMA_VERSION,
                profiles,
            };
            let persistence = Arc::new(MemoryRepository::with_state(state));

        }
        when "the application loads both profiles" {
            let agent = KeyringdApplication::new(
                crate::test_support::successful_device_keys(),
                TestPasskeyService::new(),
                persistence.clone(),

            )
            .unwrap();

        }
        then "it exposes both profiles in their persisted order" {
            let profiles = agent.profiles();
            assert_eq!(profiles.len(), 2);
            assert_eq!(profiles[0].display_name(), "Personal");
            assert_eq!(profiles[1].display_name(), "Work");
        }

    }

    case "Rejects an unsupported persisted schema version" {
        async: tokio;
        timeout: std::time::Duration::from_secs(10);

        given "a profile store using a schema version newer than this application" {
            let persistence = Arc::new(MemoryRepository::with_state(ProfilesDocument {
                schema_version: PROFILES_SCHEMA_VERSION + 1,
                profiles: Vec::new(),
            }));

        }
        when "the application is initialized" {
            let error = KeyringdApplication::new(
                crate::test_support::successful_device_keys(),
                TestPasskeyService::new(),
                persistence,

            )
            .err()
            .unwrap();

        }
        then "it reports a profile-store validation error for the unsupported version" {
            assert!(
                matches!(&error, ApplicationError::ProfileRepository(_)),
                "unexpected startup error: {error:?}"
            );
            assert!(
                error
                    .to_string()
                    .contains("unsupported profiles schema version")
            );
        }

    }

    }
}
