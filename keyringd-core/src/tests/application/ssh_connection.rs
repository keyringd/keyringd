use crate::application::OperationRejectionReason;
use crate::application::PasskeyChoice;
use crate::application::ProfileSetupRequest;
use crate::application::{ApplicationError, KeyringdApplication};
use crate::model::Profile;
use crate::model::ProfileSshIdentityKind;
use crate::service::SshConnectionTestResult;
use crate::test_support::{
    MemoryProfileRepository, RecordingOpenSshService, SoftwareDeviceKey, TestPasskeyService,
};
use std::sync::Arc;

async fn application() -> Arc<KeyringdApplication<SoftwareDeviceKey, TestPasskeyService>> {
    KeyringdApplication::new(
        SoftwareDeviceKey::generating(),
        TestPasskeyService::default(),
        Arc::new(MemoryProfileRepository::default()),
    )
    .unwrap()
}
async fn profile(
    application: &KeyringdApplication<SoftwareDeviceKey, TestPasskeyService>,
) -> Profile {
    application
        .setup_profile(
            &Default::default(),
            &Default::default(),
            ProfileSetupRequest::new("Connection test".into(), PasskeyChoice::CreateNew),
            |_| {},
            crate::test_support::reject_credential_selection,
        )
        .await
        .unwrap()
}

keyringd_testkit::scenario! {
    group "SSH connection testing" {
        case "SSH connection testing rejects disabled and deleted profiles without invoking OpenSSH" {
            async: tokio;
            timeout: std::time::Duration::from_secs(10);
            verify "the connection request uses the current profile state" {
                let app = application().await;
                let service = RecordingOpenSshService::default();
                let snapshot = profile(&app).await;
                app.set_profile_enabled(snapshot.id(), false).unwrap();
                assert!(matches!(
                    app.test_ssh_connection(
                        snapshot.id(),
                        ProfileSshIdentityKind::DeviceKey,
                        "host".into(),
                        &service
                    )
                    .await,
                    Err(ApplicationError::OperationRejected(
                        OperationRejectionReason::ProfileDisabled
                    ))
                ));
                assert!(service.requests.lock().unwrap().is_empty());
                app.remove_profile(&Default::default(), snapshot.id()).await.unwrap();
                assert!(matches!(
                    app.test_ssh_connection(
                        snapshot.id(),
                        ProfileSshIdentityKind::DeviceKey,
                        "host".into(),
                        &service
                    )
                    .await,
                    Err(ApplicationError::NotFound(_))
                ));
                assert!(service.requests.lock().unwrap().is_empty());
            }
        }

        case "SSH connection testing uses the current Device Key after replacement" {
            async: tokio;
            timeout: std::time::Duration::from_secs(10);
            verify "the connection request uses the current profile state" {
                let app = application().await;
                let old = profile(&app).await;
                app.replace_device_key(&Default::default(), old.id()).await.unwrap();
                let current = app.profile(old.id()).unwrap();
                let service = RecordingOpenSshService::default();
                let kind = ProfileSshIdentityKind::DeviceKey;
                let report = app
                    .test_ssh_connection(old.id(), kind, "user@host".into(), &service)
                    .await
                    .unwrap();
                assert_ne!(kind.trust_entry(&old), kind.trust_entry(&current));
                assert_eq!(report.trust_entry, kind.trust_entry(&current));
                assert_eq!(report.result, SshConnectionTestResult::IdentityRejected);
                assert_eq!(
                    *service.requests.lock().unwrap(),
                    vec![(
                        "user@host".into(),
                        crate::model::ProfileSshIdentity::from_profile_at(
                            &current,
                            kind,
                            std::time::SystemTime::now(),
                        )
                        .unwrap()
                        .to_public_key_line()
                    )]
                );
            }
        }
    }
}
