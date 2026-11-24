use keyringd_fido_client::HybridAdvertisementRelay;
use keyringd_ssh_agent_protocol::{
    SshAgentExtensionRequest, SshAgentRequest, SshAgentRequestHandler, SshAgentResponse,
    SshAgentSignRequest,
};
use std::sync::Arc;

use keyringd_ssh_agent_protocol::extension_names::PXP_ADVERTISEMENT_RELAY_EXTENSION;

use crate::application::KeyringdApplication;
use crate::model::{Profile, ProfileSshIdentity, ProfileSshIdentityKind};
use crate::service::{DeviceKeyService, PasskeyService};

pub struct KeyringdAgentRequestHandler<D: DeviceKeyService, F> {
    application: Arc<KeyringdApplication<D, F>>,
    device_key_context: D::OperationContext,
    remote_hybrid_advertisements: Option<HybridAdvertisementRelay>,
}

impl<D: DeviceKeyService<OperationContext = ()>, F> KeyringdAgentRequestHandler<D, F> {
    pub fn new(application: Arc<KeyringdApplication<D, F>>) -> Self {
        Self::new_with_device_key_context(application, ())
    }
}

impl<D: DeviceKeyService, F> KeyringdAgentRequestHandler<D, F> {
    pub fn new_with_device_key_context(
        application: Arc<KeyringdApplication<D, F>>,
        device_key_context: D::OperationContext,
    ) -> Self {
        Self {
            application,
            device_key_context,
            remote_hybrid_advertisements: None,
        }
    }

    pub fn with_remote_hybrid_advertisements(mut self, relay: HybridAdvertisementRelay) -> Self {
        self.remote_hybrid_advertisements = Some(relay);
        self
    }
}

impl<D, F> SshAgentRequestHandler for KeyringdAgentRequestHandler<D, F>
where
    D: DeviceKeyService + 'static,
    F: PasskeyService + 'static,
{
    async fn handle(&self, request: SshAgentRequest) -> SshAgentResponse {
        match request {
            SshAgentRequest::Identities => self.handle_identities(),
            SshAgentRequest::Sign(request) => self.handle_sign(request).await,
            SshAgentRequest::Extension(request) => self.handle_extension(request),
            SshAgentRequest::Unknown { .. } => SshAgentResponse::Failure,
        }
    }
}

impl<D, F> KeyringdAgentRequestHandler<D, F>
where
    D: DeviceKeyService + 'static,
    F: PasskeyService + 'static,
{
    fn handle_identities(&self) -> SshAgentResponse {
        let profiles = self.application.profiles();
        let reference_time = std::time::SystemTime::now();
        for profile in profiles.iter().filter(|profile| profile.is_enabled()) {
            if !profile
                .device_key()
                .certificate()
                .is_within_validity_period(reference_time)
            {
                tracing::debug!(profile = %profile.display_name(), "enabled profile has no device certificate");
            }
        }
        SshAgentResponse::IdentitiesAnswer {
            identities: agent_identities_at(&profiles, reference_time),
        }
    }

    async fn handle_sign(&self, request: SshAgentSignRequest) -> SshAgentResponse {
        let profiles = self.application.profiles();
        let now = std::time::SystemTime::now();
        for profile in &profiles {
            if !profile.is_enabled() {
                continue;
            }
            let matches_device_key = [
                ProfileSshIdentityKind::DeviceKey,
                ProfileSshIdentityKind::DeviceCertificate,
            ]
            .into_iter()
            .any(|kind| {
                ProfileSshIdentity::from_profile_at(profile, kind, now)
                    .is_some_and(|identity| identity.key_blob() == request.key_blob())
            });
            if matches_device_key {
                return match self
                    .application
                    .sign_with_device_key(&self.device_key_context, profile.id(), &request)
                    .await
                {
                    Ok(signature) => SshAgentResponse::Signature { signature },
                    Err(error) => {
                        error.log_failure("profile TPM signing was not completed");
                        SshAgentResponse::Failure
                    }
                };
            }
        }

        for profile in &profiles {
            if !profile.is_enabled() {
                continue;
            }
            let matches_passkey =
                ProfileSshIdentity::from_profile_at(profile, ProfileSshIdentityKind::Passkey, now)
                    .is_some_and(|identity| identity.key_blob() == request.key_blob());
            if !matches_passkey {
                continue;
            }
            return match self
                .application
                .sign_with_passkey(profile.id(), &request)
                .await
            {
                Ok(signature) => SshAgentResponse::Signature { signature },
                Err(error) => {
                    error.log_failure("passkey signing was not completed");
                    SshAgentResponse::Failure
                }
            };
        }

        SshAgentResponse::Failure
    }

    fn handle_extension(&self, request: SshAgentExtensionRequest) -> SshAgentResponse {
        if request.name() == PXP_ADVERTISEMENT_RELAY_EXTENSION {
            let Some(relay) = &self.remote_hybrid_advertisements else {
                return SshAgentResponse::Failure;
            };
            let data = request.data();
            if data.len() != 22
                || !matches!(u16::from_le_bytes([data[0], data[1]]), 0xfff9 | 0xfde2)
            {
                return SshAgentResponse::ExtensionFailure;
            }
            let mut service_data = [0; 20];
            service_data.copy_from_slice(&data[2..]);
            relay.publish(service_data);
            return SshAgentResponse::Success;
        }
        tracing::debug!(name = request.name(), "unsupported SSH agent extension");
        // Do not claim session-bind support until the binding is parsed,
        // verified, stored, and enforced for subsequent signatures.
        SshAgentResponse::Failure
    }
}

fn agent_identities_at(
    profiles: &[Profile],
    reference_time: std::time::SystemTime,
) -> Vec<keyringd_ssh_agent_protocol::SshAgentIdentity> {
    ProfileSshIdentityKind::ALL
        .into_iter()
        .flat_map(|kind| {
            profiles
                .iter()
                .filter(|profile| profile.is_enabled())
                .filter_map(move |profile| {
                    ProfileSshIdentity::from_profile_at(profile, kind, reference_time)
                        .map(ProfileSshIdentity::into_agent_identity)
                })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::PasskeyChoice;
    use crate::application::ProfileSetupRequest;
    use crate::test_support::{
        MemoryProfileRepository, MockDeviceKeyService, TestPasskeyService, successful_device_keys,
    };
    use keyringd_ssh_agent_protocol::{SshAgentSignFlags, SshKeyAlgorithm, SshSignatureAlgorithm};
    use keyringd_ssh_agent_protocol::{SshPublicKeyBlob, SshSigningPayload};
    async fn completed_application()
    -> Arc<KeyringdApplication<MockDeviceKeyService, TestPasskeyService>> {
        let application = KeyringdApplication::new(
            successful_device_keys(),
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
                crate::test_support::reject_credential_selection,
            )
            .await
            .unwrap();
        application
    }

    fn append_string(bytes: &mut Vec<u8>, value: &[u8]) {
        bytes.extend((value.len() as u32).to_be_bytes());
        bytes.extend(value);
    }

    fn sshsig_payload() -> Vec<u8> {
        let mut data = b"SSHSIG".to_vec();
        append_string(&mut data, b"keyringd-test");
        append_string(&mut data, b"");
        append_string(&mut data, b"sha256");
        append_string(&mut data, &[7; 32]);
        data
    }

    fn userauth_payload(algorithm: &[u8], public_key: &SshPublicKeyBlob) -> Vec<u8> {
        let mut data = Vec::new();
        append_string(&mut data, b"session-id");
        data.push(50);
        append_string(&mut data, b"user");
        append_string(&mut data, b"ssh-connection");
        append_string(&mut data, b"publickey");
        data.push(1);
        append_string(&mut data, algorithm);
        append_string(&mut data, &public_key.to_bytes());
        data
    }

    fn profile_identity<D: DeviceKeyService + 'static, F: PasskeyService + 'static>(
        application: &KeyringdApplication<D, F>,
        kind: ProfileSshIdentityKind,
    ) -> SshPublicKeyBlob
where {
        ProfileSshIdentity::from_profile_at(
            &application.profiles()[0],
            kind,
            std::time::SystemTime::now(),
        )
        .expect("the completed test profile exposes every identity kind")
        .key_blob()
        .clone()
    }

    async fn sign(
        handler: &KeyringdAgentRequestHandler<MockDeviceKeyService, TestPasskeyService>,
        key: SshPublicKeyBlob,
        data: Vec<u8>,
    ) -> SshAgentResponse {
        handler
            .handle(SshAgentRequest::Sign(SshAgentSignRequest::new(
                key,
                data,
                SshAgentSignFlags::default(),
            )))
            .await
    }

    fn algorithm(kind: ProfileSshIdentityKind) -> SshKeyAlgorithm {
        match kind {
            ProfileSshIdentityKind::DeviceKey => SshKeyAlgorithm::EcdsaSha2NistP256,
            ProfileSshIdentityKind::DeviceCertificate => {
                SshKeyAlgorithm::EcdsaSha2NistP256Certificate
            }
            ProfileSshIdentityKind::Passkey => SshKeyAlgorithm::SkEcdsaSha2NistP256,
        }
    }

    async fn accepts_signature(
        request_kind: ProfileSshIdentityKind,
        payload_kind: Option<ProfileSshIdentityKind>,
    ) {
        let app = completed_application().await;
        let key = profile_identity(&app, request_kind);
        let data = match payload_kind {
            Some(kind) => {
                userauth_payload(algorithm(kind).as_bytes(), &profile_identity(&app, kind))
            }
            None => sshsig_payload(),
        };
        let handler = KeyringdAgentRequestHandler::new(app);
        let SshAgentResponse::Signature { signature } = sign(&handler, key, data).await else {
            panic!("expected a signature");
        };
        let expected = if matches!(request_kind, ProfileSshIdentityKind::Passkey) {
            SshSignatureAlgorithm::SkEcdsaSha2NistP256
        } else {
            SshSignatureAlgorithm::EcdsaSha2NistP256
        };
        assert_eq!(signature.algorithm(), expected);
    }

    keyringd_testkit::scenario! {
        group "KeyringdAgentRequestHandler::handle" {
            group "SSH user authentication" {
                case "Accepts userauth for Device Key" {
                    async: tokio;
                    timeout: std::time::Duration::from_secs(10);
                    verify "the selected signing identity returns a signature" {
                        accepts_signature(
                            ProfileSshIdentityKind::DeviceKey,
                            Some(ProfileSshIdentityKind::DeviceKey),
                        )
                        .await;
                    }
                }

                case "Accepts userauth for Device Certificate" {
                    async: tokio;
                    timeout: std::time::Duration::from_secs(10);
                    verify "the selected signing identity returns a signature" {
                        accepts_signature(
                            ProfileSshIdentityKind::DeviceCertificate,
                            Some(ProfileSshIdentityKind::DeviceCertificate),
                        )
                        .await;
                    }
                }

                case "Accepts certificate userauth when request selects underlying Device Key" {
                    async: tokio;
                    timeout: std::time::Duration::from_secs(10);
                    verify "the selected signing identity returns a signature" {
                        accepts_signature(
                            ProfileSshIdentityKind::DeviceKey,
                            Some(ProfileSshIdentityKind::DeviceCertificate),
                        )
                        .await;
                    }
                }

                case "Accepts Device Key userauth when request selects matching certificate" {
                    async: tokio;
                    timeout: std::time::Duration::from_secs(10);
                    verify "the selected signing identity returns a signature" {
                        accepts_signature(
                            ProfileSshIdentityKind::DeviceCertificate,
                            Some(ProfileSshIdentityKind::DeviceKey),
                        )
                        .await;
                    }
                }

                case "Rejects userauth for a different signing key" {
                    async: tokio;
                    timeout: std::time::Duration::from_secs(10);
                    verify "the request returns Failure" {
                        let app = completed_application().await;
                        let handler = KeyringdAgentRequestHandler::new(Arc::clone(&app));
                        for (request_kind, payload_kind) in [
                            (
                                ProfileSshIdentityKind::Passkey,
                                ProfileSshIdentityKind::DeviceKey,
                            ),
                            (
                                ProfileSshIdentityKind::DeviceKey,
                                ProfileSshIdentityKind::Passkey,
                            ),
                            (
                                ProfileSshIdentityKind::DeviceCertificate,
                                ProfileSshIdentityKind::Passkey,
                            ),
                        ] {
                            let data = userauth_payload(
                                algorithm(payload_kind).as_bytes(),
                                &profile_identity(&app, payload_kind),
                            );
                            // A valid payload ensures this exercises authorization rather than parser rejection.
                            assert!(SshSigningPayload::from_bytes(&data).is_ok());
                            assert!(matches!(
                                sign(&handler, profile_identity(&app, request_kind), data).await,
                                SshAgentResponse::Failure
                            ));
                        }
                    }
                }
            }
            group "SSHSIG signing" {
                case "Accepts SSHSIG for Device Key" {
                    async: tokio;
                    timeout: std::time::Duration::from_secs(10);
                    verify "the selected signing identity returns a signature" {
                        accepts_signature(ProfileSshIdentityKind::DeviceKey, None).await;
                    }
                }

                case "Accepts SSHSIG for Device Certificate" {
                    async: tokio;
                    timeout: std::time::Duration::from_secs(10);
                    verify "the selected signing identity returns a signature" {
                        accepts_signature(ProfileSshIdentityKind::DeviceCertificate, None).await;
                    }
                }

                case "Accepts SSHSIG for Passkey" {
                    async: tokio;
                    timeout: std::time::Duration::from_secs(10);
                    verify "the selected signing identity returns a signature" {
                        accepts_signature(ProfileSshIdentityKind::Passkey, None).await;
                    }
                }
            }
            group "Signing payload validation" {
                case "Rejects unsupported signing payload" {
                    async: tokio;
                    timeout: std::time::Duration::from_secs(10);
                    verify "the request returns Failure" {
                        let app = completed_application().await;
                        let handler = KeyringdAgentRequestHandler::new(Arc::clone(&app));
                        for kind in ProfileSshIdentityKind::ALL {
                            assert!(matches!(
                                sign(
                                    &handler,
                                    profile_identity(&app, kind),
                                    b"arbitrary bytes".to_vec()
                                )
                                .await,
                                SshAgentResponse::Failure
                            ));
                        }
                    }
                }

                case "Rejects userauth with inconsistent algorithm" {
                    async: tokio;
                    timeout: std::time::Duration::from_secs(10);
                    verify "the request returns Failure" {
                        let app = completed_application().await;
                        let key = profile_identity(&app, ProfileSshIdentityKind::Passkey);
                        let data = userauth_payload(
                            SshKeyAlgorithm::SkEcdsaSha2NistP256Certificate.as_bytes(),
                            &key,
                        );
                        let handler = KeyringdAgentRequestHandler::new(app);
                        assert!(matches!(
                            sign(&handler, key, data).await,
                            SshAgentResponse::Failure
                        ));
                    }
                }
            }
            group "Disabled profiles" {
                case "Hides identities for disabled profile" {
                    async: tokio;
                    timeout: std::time::Duration::from_secs(10);
                    verify "the identity list is empty" {
                        let app = completed_application().await;
                        let id = app.profiles()[0].id().clone();
                        app.set_profile_enabled(&id, false).unwrap();
                        let handler = KeyringdAgentRequestHandler::new(app);
                        let SshAgentResponse::IdentitiesAnswer { identities } =
                            handler.handle(SshAgentRequest::Identities).await
                        else {
                            panic!("expected identities");
                        };
                        assert!(identities.is_empty());
                    }
                }

                case "Rejects signing for disabled profile" {
                    async: tokio;
                    timeout: std::time::Duration::from_secs(10);
                    verify "the request returns Failure" {
                        let app = completed_application().await;
                        let keys: Vec<_> = ProfileSshIdentityKind::ALL
                            .into_iter()
                            .map(|kind| profile_identity(&app, kind))
                            .collect();
                        let id = app.profiles()[0].id().clone();
                        app.set_profile_enabled(&id, false).unwrap();
                        let handler = KeyringdAgentRequestHandler::new(app);
                        for key in keys {
                            assert!(matches!(
                                sign(&handler, key, sshsig_payload()).await,
                                SshAgentResponse::Failure
                            ));
                        }
                    }
                }
            }
        }
    }
}
