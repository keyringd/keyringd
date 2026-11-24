pub(super) use std::sync::Arc;

pub(super) use crate::application::ApplicationError;
pub(super) use crate::application::KeyringdApplication;
pub(super) use crate::model::ProfileId;
pub(super) use crate::test_support::MockDeviceKeyService;
pub(super) use crate::test_support::failing_assertion_fido;
pub(super) use crate::test_support::{TrackedDeviceKeyOperationContext, TrackingDeviceKeyService};

pub(super) use crate::model::device_key::{DevicePrivateKeyHandle, LocalAuthentication};
pub(super) use crate::model::passkey::Passkey;
pub(super) use crate::service::DeviceKeyService;
pub(super) use crate::service::PasskeyService;
pub(super) use crate::test_support::TestPasskeyService;

pub(super) use crate::application::PasskeyChoice;
pub(super) use crate::application::ProfileSetupRequest;
pub(super) use crate::model::Profile;
pub(super) use crate::persistence::{PROFILES_SCHEMA_VERSION, ProfilesDocument};
pub(super) use crate::test_support::MemoryProfileRepository as MemoryRepository;

pub(super) async fn completed_application() -> (
    Arc<KeyringdApplication<TrackingDeviceKeyService, TestPasskeyService>>,
    Arc<MemoryRepository>,
    TrackingDeviceKeyService,
    ProfileId,
) {
    let store = Arc::new(MemoryRepository::default());
    let device_keys = TrackingDeviceKeyService::default();
    let application = KeyringdApplication::new(
        device_keys.clone(),
        TestPasskeyService::new(),
        store.clone(),
    )
    .unwrap();
    let id = setup_profile(&application, "Default").await;
    (application, store, device_keys, id)
}

pub(super) async fn completed_application_with_journal() -> (
    Arc<KeyringdApplication<TrackingDeviceKeyService, TestPasskeyService>>,
    Arc<MemoryRepository>,
    TrackingDeviceKeyService,
    ProfileId,
    crate::test_support::OperationJournal,
) {
    use crate::test_support::OperationJournal;

    let journal = OperationJournal::default();
    let store = Arc::new(MemoryRepository::with_journal(journal.clone()));
    let device_keys = TrackingDeviceKeyService::with_journal(journal.clone());
    let application = KeyringdApplication::new(
        device_keys.clone(),
        TestPasskeyService::new(),
        store.clone(),
    )
    .unwrap();
    let id = setup_profile(&application, "Default").await;
    journal.clear();
    (application, store, device_keys, id, journal)
}

pub(super) async fn application_with_two_profiles() -> (
    Arc<KeyringdApplication<TrackingDeviceKeyService, TestPasskeyService>>,
    Arc<MemoryRepository>,
    ProfileId,
    ProfileId,
) {
    let (application, store, _device_keys, first_id) = completed_application().await;
    let second_id = setup_profile(&application, "Work").await;
    (application, store, first_id, second_id)
}

pub(super) async fn setup_profile<D, F>(
    application: &KeyringdApplication<D, F>,
    display_name: &str,
) -> ProfileId
where
    D: DeviceKeyService + 'static,
    D::OperationContext: Default,
    F: PasskeyService + 'static,
{
    application
        .setup_profile(
            &Default::default(),
            &Default::default(),
            ProfileSetupRequest::new(display_name.to_owned(), PasskeyChoice::CreateNew),
            |_| {},
            crate::test_support::reject_credential_selection,
        )
        .await
        .unwrap()
        .id()
        .clone()
}

pub(super) fn passkey_make_options(
    challenge: &[u8],
) -> (
    keyringd_fido::webauthn::PublicKeyCredentialCreationOptions,
    std::sync::Arc<dyn keyringd_fido_client::ClientDataProvider>,
) {
    use keyringd_fido::webauthn::{
        AuthenticatorSelectionCriteria, PublicKeyCredentialCreationOptions,
        PublicKeyCredentialParameters, PublicKeyCredentialRpEntity, PublicKeyCredentialType,
        PublicKeyCredentialUserEntity,
    };
    use keyringd_fido::{CollectedClientData, ResidentKeyRequirement, UserVerificationRequirement};
    use keyringd_fido_client::{ClientData, ClientDataProviderError};

    (
        PublicKeyCredentialCreationOptions::new(
            PublicKeyCredentialRpEntity::new("Keyringd").with_id(Passkey::RP_ID),
            PublicKeyCredentialUserEntity::new("keyringd-test", vec![7; 32], "keyringd test"),
            challenge.to_vec(),
            vec![PublicKeyCredentialParameters::new(
                PublicKeyCredentialType::PublicKey,
                keyringd_fido::webauthn::CoseAlgorithmIdentifier::ES256,
            )],
        )
        .with_timeout(1_000)
        .with_authenticator_selection(
            AuthenticatorSelectionCriteria::default()
                .with_resident_key(ResidentKeyRequirement::Required)
                .with_require_resident_key(true)
                .with_user_verification(UserVerificationRequirement::Preferred),
        ),
        std::sync::Arc::new(|challenge: &[u8]| {
            CollectedClientData::create(challenge, Passkey::WEB_AUTHN_ORIGIN)
                .and_then(ClientData::try_from)
                .map_err(ClientDataProviderError::new)
        }),
    )
}

pub(super) async fn persisted_profile(
    id: ProfileId,
    display_name: &str,
    handle: DevicePrivateKeyHandle,
) -> Profile {
    let store = Arc::new(MemoryRepository::default());
    let agent = KeyringdApplication::new(
        crate::test_support::successful_device_keys(),
        TestPasskeyService::new(),
        store.clone(),
    )
    .unwrap();
    setup_profile(&agent, "Default").await;
    let profile = store.snapshot().unwrap().profiles.remove(0);
    let key = profile.device_key();
    Profile::new(
        id,
        display_name.to_owned(),
        false,
        crate::model::device_key::DeviceKey::new(
            handle,
            key.certificate().clone(),
            key.local_authentication(),
            key.signature_algorithm(),
        )
        .unwrap(),
        profile.passkey().clone(),
    )
    .unwrap()
}

pub(super) async fn seed_default_profile(handle: &[u8]) -> (Arc<MemoryRepository>, ProfileId) {
    let id = ProfileId::from_stored("default".to_string());
    let state = ProfilesDocument {
        schema_version: PROFILES_SCHEMA_VERSION,
        profiles: vec![
            persisted_profile(
                id.clone(),
                "Default",
                DevicePrivateKeyHandle::new(handle.to_vec()),
            )
            .await,
        ],
    };
    (Arc::new(MemoryRepository::with_state(state)), id)
}

pub(super) use keyringd_fido::webauthn::PublicKeyCredentialHint;
