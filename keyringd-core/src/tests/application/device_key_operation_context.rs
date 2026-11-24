use super::fixture::*;

async fn application_with_context_tracking() -> (
    Arc<KeyringdApplication<TrackingDeviceKeyService, TestPasskeyService>>,
    TrackingDeviceKeyService,
) {
    let device_keys = TrackingDeviceKeyService::default();
    let application = KeyringdApplication::new(
        device_keys.clone(),
        TestPasskeyService::new(),
        Arc::new(MemoryRepository::default()),
    )
    .unwrap();
    (application, device_keys)
}

keyringd_testkit::scenario! {
    group "KeyringdApplication::setup_profile" {
        case "Forwards the explicitly supplied Device Key operation context" {
            why "Device Key operations must forward their caller-supplied context so native security UI is owned by the initiating window."
            async: tokio;
            timeout: std::time::Duration::from_secs(10);

            given "an application tracking Device Key operation contexts" {
                let (application, device_keys) = application_with_context_tracking()
                .await;
            }
            when "profile setup is run with a caller-supplied Device Key operation context" {
                application
                    .setup_profile(
                        &TrackedDeviceKeyOperationContext::SuppliedByCaller, &Default::default(),
                        ProfileSetupRequest::new("Default".to_owned(), PasskeyChoice::CreateNew),
                        |_| {},
                        crate::test_support::reject_credential_selection,
                    )
                    .await
                    .unwrap();
            }
            then "the Device Key service receives the caller-supplied context" {
                assert_eq!(
                    device_keys.operation_contexts(),
                    vec![TrackedDeviceKeyOperationContext::SuppliedByCaller]
                );
            }
        }
    }
}
