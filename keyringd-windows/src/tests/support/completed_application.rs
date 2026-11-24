use std::sync::Arc;

use keyringd_core::application::KeyringdApplication;
use keyringd_core::application::PasskeyChoice;
use keyringd_core::application::ProfileSetupRequest;
use keyringd_core::test_support::TestPasskeyService;

use super::MemoryProfileRepository;
use super::SoftwareDeviceKey;

pub async fn completed_application(
    seed: u8,
    handle: &'static [u8],
) -> Arc<KeyringdApplication<SoftwareDeviceKey, TestPasskeyService>> {
    let application = KeyringdApplication::new(
        SoftwareDeviceKey::new(seed, handle),
        TestPasskeyService::new(),
        Arc::new(MemoryProfileRepository::default()),
    )
    .unwrap();
    application
        .setup_profile(
            &Default::default(),
            &Default::default(),
            ProfileSetupRequest::new("Default".to_owned(), PasskeyChoice::CreateNew),
            |_| {},
            keyringd_core::test_support::reject_credential_selection,
        )
        .await
        .unwrap();
    application
}
