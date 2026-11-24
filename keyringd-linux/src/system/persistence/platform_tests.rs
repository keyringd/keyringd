use crate::system::test_support::PlatformFixture;
use keyringd_core::model::ssh_trust::{SshCertificateAuthorityTrustEntry, SshDeviceKeyTrustEntry};

keyringd_testkit::scenario! {
    case "Restore Completed Profile from Platform Store and TPM" {
        async: tokio;
        timeout: std::time::Duration::from_secs(10);

        given "Prepare input and initial state to validate \"Restore Completed Profile from Platform Store and TPM\"" {
            let fixture = PlatformFixture::new().await;
            let id = fixture.complete_initial_profile().await;

        }
        when "Call Fixture.Reopen_Application" {
            let restored = fixture.reopen_application().await;

        }
        then "Check the results and side effects of \"Restore Completed Profile from Platform Store and TPM\"" {
            assert_eq!(restored.profiles().len(), 1);
            assert_eq!(restored.profiles()[0].id(), &id);
            assert!(restored.profiles()[0].device_key().certificate().is_within_validity_period(std::time::SystemTime::now()));
        }

    }
}

keyringd_testkit::scenario! {
    case "Maintain CA Trust with an explicit Device Key substitution" {
        async: tokio;
        timeout: std::time::Duration::from_secs(10);

        given "Prepare inputs and initial states to validate \"keep ca trust with explicit device key replacement\"" {
            let fixture = PlatformFixture::new().await;
            let id = fixture.complete_initial_profile().await;
            let application = &fixture.application;
            let original = application.profiles()[0].clone();

        }
        when "Call application.replace_device_key" {
            application.replace_device_key(&Default::default(), &id).await.unwrap();

        }
        then "See results and side effects of \"Keep CA Trust with Explicit Device Key Replacement\"" {
            let replaced = &application.profiles()[0];
            assert_ne!(
                SshDeviceKeyTrustEntry::from_profile(replaced),
                SshDeviceKeyTrustEntry::from_profile(&original)
            );
            assert_eq!(
                SshCertificateAuthorityTrustEntry::from_profile(replaced),
                SshCertificateAuthorityTrustEntry::from_profile(&original)
            );
        }

    }
}
