use super::DeviceKeyReadyProfileDraft;
use super::PasskeyReadyProfileDraft;
use super::ProfileDraft;
use crate::application::PasskeyCredentialCandidate;
use crate::application::ProfileSetupRequest;
use crate::application::{ApplicationError, OperationRejectionReason, ProfileStore};
use crate::application::{
    PasskeyChoice, PasskeyCredentialSelector, ProfileSetupFailure, ProfileSetupProgress,
    ProfileSetupProgressReporter,
};
use crate::execution::ExecutionScope;
use crate::execution::operation::FidoOperationTarget;
use crate::execution::util::DeviceKeyCertificateIssuer;
use crate::execution::util::delete_key_best_effort;
use crate::model::Profile;
use crate::model::ProfileId;
use crate::model::device_key::DeviceKey;
use crate::model::device_key::DeviceKeyAlgorithm;
use crate::model::device_key::LocalAuthentication;
use crate::model::passkey::Challenge;
use crate::model::passkey::CredentialId;
use crate::model::passkey::Passkey;
use crate::model::passkey::PasskeyAttestationResponse;
use crate::service::DeviceKeyGenerationReason;
use crate::service::DeviceKeyGenerationRequest;
use crate::service::DeviceKeyMaterial;
use crate::service::{DeviceKeyService, PasskeyService, SerializedPasskeyService};
use crate::signing::fido::{PasskeyAssertionProvider, passkey_assertion_options};
use crate::util::crypto;
use keyringd_fido::webauthn::AuthenticationExtensionsClientInputs;
use keyringd_fido::webauthn::CoseAlgorithmIdentifier;
use keyringd_fido::webauthn::LargeBlobOutput;
use keyringd_fido::webauthn::{
    AuthenticatorSelectionCriteria, LargeBlobInput, LargeBlobSupport,
    PublicKeyCredentialCreationOptions, PublicKeyCredentialParameters, PublicKeyCredentialRpEntity,
    PublicKeyCredentialType, PublicKeyCredentialUserEntity,
};
use keyringd_fido::{CollectedClientData, ResidentKeyRequirement, UserVerificationRequirement};
use keyringd_fido_client::{ClientData, GetAssertionResult};
use keyringd_ssh_agent_protocol::SshSignatureAlgorithm;
use tokio_util::sync::CancellationToken;

/// Completes one profile registration, including passkey selection and cleanup of uncommitted keys.
pub(crate) struct SetupProfileExecution<'a, D: DeviceKeyService, F> {
    scope: ExecutionScope,
    device_key_service: &'a D,
    passkey_service: &'a SerializedPasskeyService<F>,
    profiles: &'a ProfileStore,
}

impl<'a, D: DeviceKeyService, F> SetupProfileExecution<'a, D, F> {
    pub(crate) fn new(
        device_key_service: &'a D,
        passkey_service: &'a SerializedPasskeyService<F>,
        profiles: &'a ProfileStore,
    ) -> Self {
        Self {
            scope: ExecutionScope::new("setup_profile"),
            device_key_service,
            passkey_service,
            profiles,
        }
    }
}

impl<D: DeviceKeyService, F: PasskeyService> SetupProfileExecution<'_, D, F> {
    pub(crate) async fn execute(
        self,
        context: &D::OperationContext,
        cancellation: &CancellationToken,
        request: ProfileSetupRequest,
        progress_reporter: impl ProfileSetupProgressReporter,
        credential_selector: impl PasskeyCredentialSelector,
    ) -> Result<Profile, ProfileSetupFailure> {
        self.scope
            .run(self.execute_inner(
                context,
                cancellation,
                request,
                progress_reporter,
                credential_selector,
            ))
            .await
            .map_err(ProfileSetupFailure::from)
    }

    async fn execute_inner(
        &self,
        context: &D::OperationContext,
        cancellation: &CancellationToken,
        request: ProfileSetupRequest,
        mut progress_reporter: impl ProfileSetupProgressReporter,
        credential_selector: impl PasskeyCredentialSelector,
    ) -> Result<Profile, ApplicationError> {
        progress_reporter.report_progress(ProfileSetupProgress::Starting);
        check_cancellation(cancellation)?;
        let draft = self.prepare_profile_draft(&request)?;
        let mut fido_target = FidoOperationTarget::Unselected;
        let passkey_ready = match request.passkey {
            PasskeyChoice::CreateNew => {
                progress_reporter.report_progress(ProfileSetupProgress::CreatingPasskey);
                check_cancellation(cancellation)?;
                self.create_passkey(cancellation, &mut fido_target, draft)
                    .await?
            }
            PasskeyChoice::Existing => {
                self.recover_passkey(
                    &mut fido_target,
                    draft,
                    cancellation,
                    &mut progress_reporter,
                    credential_selector,
                )
                .await?
            }
        };
        progress_reporter.report_progress(ProfileSetupProgress::ProvisioningDeviceKey);
        check_cancellation(cancellation)?;
        self.provision_and_commit_profile(
            context,
            cancellation,
            &mut fido_target,
            passkey_ready,
            request.passkey,
            &mut progress_reporter,
        )
        .await
    }

    fn prepare_profile_draft(
        &self,
        request: &ProfileSetupRequest,
    ) -> Result<ProfileDraft, ApplicationError> {
        Passkey::validate_signature_algorithm(&request.passkey_signature_algorithm)
            .map_err(|_| OperationRejectionReason::UnsupportedPasskeySignatureAlgorithm)?;
        DeviceKey::validate_signature_algorithm(&request.device_key_signature_algorithm)
            .map_err(|_| OperationRejectionReason::UnsupportedDeviceKeySignatureAlgorithm)?;
        let name = request.display_name.trim();
        Profile::validate_display_name(name)
            .map_err(|_| OperationRejectionReason::InvalidProfileName)?;
        Ok(ProfileDraft::new(
            ProfileId::new(),
            name.to_string(),
            LocalAuthentication::NotRequired,
            request.passkey_signature_algorithm.clone(),
            request.device_key_signature_algorithm.clone(),
        ))
    }

    async fn create_passkey(
        &self,
        cancellation: &CancellationToken,
        fido_target: &mut FidoOperationTarget,
        draft: ProfileDraft,
    ) -> Result<PasskeyReadyProfileDraft, ApplicationError> {
        self.passkey_service
            .run_with_cancellable_wait(cancellation, |passkey_service| async {
                let challenge = Challenge::generate().into_bytes();
                let expected_client_data =
                    CollectedClientData::create(&challenge, Passkey::WEB_AUTHN_ORIGIN)
                        .and_then(ClientData::try_from)
                        .map_err(|error| {
                            ApplicationError::InvalidAuthenticatorResponse(error.into())
                        })?;
                let request_client_data = expected_client_data.clone();
                let user = new_user_entity();
                let result = passkey_service
                    .make_credential(
                        PublicKeyCredentialCreationOptions::new(
                            PublicKeyCredentialRpEntity::new("Keyringd").with_id(Passkey::RP_ID),
                            user.clone(),
                            challenge,
                            vec![PublicKeyCredentialParameters::new(
                                PublicKeyCredentialType::PublicKey,
                                CoseAlgorithmIdentifier::ES256,
                            )],
                        )
                        .with_extensions(
                            AuthenticationExtensionsClientInputs::default().with_large_blob(
                                LargeBlobInput::request_support(LargeBlobSupport::Preferred),
                            ),
                        )
                        .with_timeout(300_000)
                        .with_authenticator_selection(
                            AuthenticatorSelectionCriteria::default()
                                .with_resident_key(ResidentKeyRequirement::Required)
                                .with_require_resident_key(true)
                                .with_user_verification(UserVerificationRequirement::Required),
                        )
                        .with_hints(fido_target.hints()),
                        std::sync::Arc::new(move |_: &[u8]| Ok(request_client_data.clone())),
                    )
                    .await?;
                let credential = result.credential();
                if credential.response().client_data_json() != expected_client_data.as_bytes() {
                    return Err(ApplicationError::InvalidAuthenticatorResponse(
                        "authenticator result does not match the requested client data".into(),
                    ));
                }
                fido_target.remember(
                    credential
                        .response()
                        .transports()
                        .iter()
                        .find(|transport| transport.as_str() == "usb")
                        .or_else(|| credential.response().transports().first())
                        .expect("validated transport"),
                    result.usb_device_id(),
                )?;
                let authenticator_data =
                    credential
                        .response()
                        .authenticator_data()
                        .map_err(|error| {
                            ApplicationError::InvalidAuthenticatorResponse(error.into())
                        })?;
                let attested = authenticator_data
                    .attested_credential_data()
                    .ok_or_else(|| {
                        ApplicationError::InvalidAuthenticatorResponse(
                            "authenticator omitted attested credential data".into(),
                        )
                    })?;
                let attestation = PasskeyAttestationResponse::new(
                    credential.response().attestation_object().to_vec(),
                    credential.response().client_data_json().to_vec(),
                )
                .map_err(|e| ApplicationError::InvalidAuthenticatorResponse(e.into()))?;
                let passkey = Passkey::new(
                    draft.passkey_signature_algorithm(),
                    CredentialId::new(credential.raw_id().to_vec()),
                    user.id().to_vec(),
                    attested
                        .credential_public_key()
                        .es256_sec1_public_key()
                        .map_err(|error| {
                            ApplicationError::InvalidAuthenticatorResponse(error.into())
                        })?
                        .to_vec(),
                )
                .map_err(|error| ApplicationError::InvalidAuthenticatorResponse(error.into()))?;
                let passkey = passkey
                    .with_attestation_response(attestation)
                    .map_err(|e| ApplicationError::InvalidAuthenticatorResponse(e.into()))?;
                Ok(draft.with_passkey(passkey))
            })
            .await
    }

    async fn recover_passkey(
        &self,
        fido_target: &mut FidoOperationTarget,
        draft: ProfileDraft,
        cancellation: &CancellationToken,
        progress_reporter: &mut impl ProfileSetupProgressReporter,
        mut credential_selector: impl PasskeyCredentialSelector,
    ) -> Result<PasskeyReadyProfileDraft, ApplicationError> {
        let challenge = Challenge::generate();
        progress_reporter.report_progress(ProfileSetupProgress::DiscoveringPasskey);
        check_cancellation(cancellation)?;
        let assertions = self
            .passkey_service
            .run_with_cancellable_wait(cancellation, |passkey_service| async {
                let options = passkey_assertion_options(challenge.as_bytes(), &fido_target.hints())
                    .with_extensions(
                        AuthenticationExtensionsClientInputs::default()
                            .with_large_blob(LargeBlobInput::request_read()),
                    );
                PasskeyAssertionProvider::new(passkey_service, fido_target)
                    .get_assertions(
                        options,
                        std::sync::Arc::new(|challenge: &[u8]| {
                            Ok(ClientData::from_bytes(challenge.to_vec()))
                        }),
                    )
                    .await
            })
            .await?;
        if assertions.responses().is_empty() {
            return Err(ApplicationError::PasskeyCredentialNotFound);
        }
        let selected = if assertions.responses().len() == 1 {
            0
        } else if let Some(index) = assertions
            .responses()
            .iter()
            .position(|response| response.user_selected() == Some(true))
        {
            index
        } else {
            let mut candidates = assertions
                .responses()
                .iter()
                .map(PasskeyCredentialCandidate::from_assertion)
                .collect::<Vec<_>>();
            PasskeyCredentialCandidate::make_labels_unique(&mut candidates);
            let id = tokio::select! {
                biased;
                _ = cancellation.cancelled() => return Err(ApplicationError::OperationCanceled),
                result = credential_selector.select_credential(candidates) => result?,
            };
            assertions
                .responses()
                .iter()
                .position(|response| response.credential().id() == id.as_bytes())
                .ok_or(ApplicationError::OperationRejected(
                    OperationRejectionReason::PasskeySelectionNotOffered,
                ))?
        };
        let assertion =
            assertions.select(Some(assertions.responses()[selected].credential().id()))?;
        check_cancellation(cancellation)?;
        let recovered_attestation = match assertion.response().large_blob() {
            Some(LargeBlobOutput::Read(Some(bytes))) => PasskeyAttestationResponse::from_large_blob(bytes)
                .and_then(|attestation| {
                    attestation
                        .verify_assertion(&assertion)
                        .map(|key| (attestation, key))
                })
                .map(Some)
                .unwrap_or_else(|error| {
                    tracing::warn!(%error, "ignoring unusable Passkey attestation from largeBlob");
                    None
                }),
            _ => None,
        };
        let (attestation, public_key) = if let Some((attestation, key)) = recovered_attestation {
            (Some(attestation), key)
        } else {
            progress_reporter.report_progress(ProfileSetupProgress::RecoveringPasskeyPublicKey);
            (
                None,
                self.recover_public_key(cancellation, fido_target, &assertion)
                    .await?,
            )
        };
        let user_handle = assertion
            .response()
            .user()
            .map(|user| user.id().to_vec())
            .ok_or_else(|| {
                ApplicationError::InvalidAuthenticatorResponse(
                    "passkey credential did not return a user handle".into(),
                )
            })?;
        if user_handle.len() != Passkey::USER_HANDLE_LEN {
            return Err(ApplicationError::InvalidAuthenticatorResponse(
                "passkey credential uses an unsupported user identity".into(),
            ));
        }
        let credential = assertion.credential();
        let passkey = Passkey::new(
            draft.passkey_signature_algorithm(),
            CredentialId::new(credential.id().to_vec()),
            user_handle,
            public_key,
        )
        .map_err(|error| ApplicationError::InvalidAuthenticatorResponse(error.into()))?;
        let passkey = if let Some(attestation) = attestation {
            passkey
                .with_attestation_response(attestation)
                .map_err(|e| ApplicationError::InvalidAuthenticatorResponse(e.into()))?
        } else {
            passkey
        };
        Ok(draft.with_passkey(passkey))
    }

    async fn recover_public_key(
        &self,
        cancellation: &CancellationToken,
        fido_target: &mut FidoOperationTarget,
        assertion1: &GetAssertionResult,
    ) -> Result<Vec<u8>, ApplicationError> {
        self.passkey_service
            .run_with_cancellable_wait(cancellation, |passkey_service| async {
                let signed_data1 = assertion1.signed_data();
                let candidates =
                    crypto::candidates(&signed_data1, assertion1.response().signature())
                        .map_err(|error| ApplicationError::Passkey(Box::new(error)))?;
                let challenge2 = Challenge::generate();
                let credential = assertion1.credential();
                let options =
                    passkey_assertion_options(challenge2.as_bytes(), &fido_target.hints())
                        .with_allow_credentials(vec![credential.clone()]);
                let assertions = PasskeyAssertionProvider::new(passkey_service, fido_target)
                    .get_assertions(
                        options,
                        std::sync::Arc::new(|challenge: &[u8]| {
                            Ok(ClientData::from_bytes(challenge.to_vec()))
                        }),
                    )
                    .await?;
                let assertion2 = assertions.select(Some(credential.id()))?;
                let signed_data2 = assertion2.signed_data();
                crypto::identify(
                    &candidates,
                    &signed_data2,
                    assertion2.response().signature(),
                )
                .map_err(|error| ApplicationError::Passkey(Box::new(error)))
            })
            .await
    }

    async fn provision_and_commit_profile(
        &self,
        context: &D::OperationContext,
        cancellation: &CancellationToken,
        fido_target: &mut FidoOperationTarget,
        draft: PasskeyReadyProfileDraft,
        passkey_choice: PasskeyChoice,
        progress_reporter: &mut impl ProfileSetupProgressReporter,
    ) -> Result<Profile, ApplicationError> {
        let DeviceKeyMaterial { handle, public_key } = self
            .device_key_service
            .generate_key(
                context,
                DeviceKeyGenerationRequest {
                    profile_name: draft.display_name(),
                    reason: DeviceKeyGenerationReason::InitialSetup,
                    algorithm: DeviceKeyAlgorithm::EcdsaP256,
                    local_authentication: draft.device_key_local_authentication(),
                },
            )
            .await
            .map_err(|error| {
                ApplicationError::from_device_key_error(self.device_key_service, error)
            })?;
        let result = async {
            let certificate = self
                .passkey_service
                .run_with_cancellable_wait(cancellation, |passkey_service| async {
                    let application = (draft.device_key_signature_algorithm()
                        != SshSignatureAlgorithm::EcdsaSha2NistP256)
                        .then_some(Passkey::RP_ID);
                    let mut issuer = DeviceKeyCertificateIssuer::new(
                        PasskeyAssertionProvider::new(passkey_service, fido_target),
                        draft.passkey(),
                    );
                    match passkey_choice {
                        PasskeyChoice::CreateNew => {
                            issuer
                                .issue_and_try_write_attestation(&public_key, application)
                                .await
                        }
                        PasskeyChoice::Existing => issuer.issue(&public_key, application).await,
                    }
                })
                .await?;
            let device_key = DeviceKey::new(
                handle.clone(),
                certificate,
                draft.device_key_local_authentication(),
                draft.device_key_signature_algorithm(),
            )
            .map_err(|error| ApplicationError::InvariantViolation(error.into()))?;
            let provisioned = draft.with_device_key(device_key);
            progress_reporter.report_progress(ProfileSetupProgress::CommittingProfile);
            self.commit_profile(cancellation, provisioned).await
        }
        .await;
        if let Err(error) = &result
            && !matches!(error, ApplicationError::ProfileDurabilityUncertain(_))
        {
            delete_key_best_effort(self.device_key_service, context, &handle).await;
        }
        result
    }

    async fn commit_profile(
        &self,
        cancellation: &CancellationToken,
        draft: DeviceKeyReadyProfileDraft,
    ) -> Result<Profile, ApplicationError> {
        let mut update = tokio::select! {
            biased;
            _ = cancellation.cancelled() => return Err(ApplicationError::OperationCanceled),
            update = self.profiles.begin_update() => update,
        };
        check_cancellation(cancellation)?;
        let profile = draft
            .complete()
            .map_err(|error| ApplicationError::InvariantViolation(error.into()))?;
        update.add_profile(profile.clone());
        update.commit()?;
        Ok(profile)
    }
}

fn check_cancellation(cancellation: &CancellationToken) -> Result<(), ApplicationError> {
    if cancellation.is_cancelled() {
        Err(ApplicationError::OperationCanceled)
    } else {
        Ok(())
    }
}

fn new_user_entity() -> PublicKeyCredentialUserEntity {
    let created_at = time::OffsetDateTime::now_local()
        .expect("the local UTC offset must be available")
        .format(time::macros::format_description!(
            "[year]-[month]-[day] [hour]:[minute]:[second]"
        ))
        .expect("the Passkey timestamp format must be valid");
    let mut user_handle = [0; Passkey::USER_HANDLE_LEN];
    getrandom::fill(&mut user_handle)
        .expect("the operating system random source must be available");
    user_entity_at(user_handle, &created_at)
}

fn user_entity_at(
    user_handle: [u8; Passkey::USER_HANDLE_LEN],
    created_at: &str,
) -> PublicKeyCredentialUserEntity {
    let display_name = format!("Keyringd — {created_at}");
    PublicKeyCredentialUserEntity::new(display_name.clone(), user_handle.to_vec(), display_name)
}
