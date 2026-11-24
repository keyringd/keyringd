use super::signing_profile;
use crate::application::{ApplicationError, ProfileStore};
use crate::execution::ExecutionScope;
use crate::model::{ProfileId, ProfileSshIdentityKind};
use crate::service::DeviceKeyService;
use keyringd_ssh_agent_protocol::SshSignatureAlgorithm;
use keyringd_ssh_agent_protocol::{SshAgentSignRequest, SshSignature};

pub(crate) struct SignWithDeviceKeyExecution<'a, D: DeviceKeyService> {
    scope: ExecutionScope,
    device_key_service: &'a D,
    profiles: &'a ProfileStore,
}

impl<'a, D: DeviceKeyService> SignWithDeviceKeyExecution<'a, D> {
    pub(crate) fn new(device_key_service: &'a D, profiles: &'a ProfileStore) -> Self {
        Self {
            scope: ExecutionScope::new("sign_with_device_key"),
            device_key_service,
            profiles,
        }
    }
}

impl<D> SignWithDeviceKeyExecution<'_, D>
where
    D: DeviceKeyService,
{
    pub(crate) async fn execute(
        self,
        context: &D::OperationContext,
        profile_id: &ProfileId,
        request: &SshAgentSignRequest,
    ) -> Result<SshSignature, ApplicationError> {
        self.scope
            .run(self.execute_inner(context, profile_id, request))
            .await
    }

    async fn execute_inner(
        self,
        context: &D::OperationContext,
        profile_id: &ProfileId,
        request: &SshAgentSignRequest,
    ) -> Result<SshSignature, ApplicationError> {
        use crate::signing::fido::{DeviceKeyAssertionProvider, DeviceKeyFidoAuthenticator};
        use crate::signing::ssh::{
            EcdsaSshSigner, SkEcdsaSshSigner, SshSigner, WebAuthnSkEcdsaSshSigner,
        };
        let profile = signing_profile(
            self.profiles,
            profile_id,
            request,
            &[
                ProfileSshIdentityKind::DeviceKey,
                ProfileSshIdentityKind::DeviceCertificate,
            ],
        )?;
        let device_key = profile.device_key();
        let profile_name = profile.display_name();
        let data = request.data();
        match device_key.signature_algorithm() {
            keyringd_ssh_agent_protocol::SshSignatureAlgorithm::EcdsaSha2NistP256 => {
                EcdsaSshSigner {
                    service: self.device_key_service,
                    context,
                    handle: device_key.handle(),
                    profile_name,
                }
                .sign(data)
                .await
            }
            format => {
                let authenticator = DeviceKeyFidoAuthenticator::new(
                    self.device_key_service,
                    device_key,
                    context,
                    device_key
                        .relying_party_id()
                        .expect("SK DeviceKey has an application"),
                    profile_name,
                );
                let provider = DeviceKeyAssertionProvider::new(authenticator);
                match format {
                    SshSignatureAlgorithm::SkEcdsaSha2NistP256 => {
                        SkEcdsaSshSigner::new(provider).sign(data).await
                    }
                    SshSignatureAlgorithm::WebAuthnSkEcdsaSha2NistP256 => {
                        WebAuthnSkEcdsaSshSigner::new(
                            provider,
                            crate::model::passkey::Passkey::WEB_AUTHN_ORIGIN.to_owned(),
                        )
                        .sign(data)
                        .await
                    }
                    SshSignatureAlgorithm::EcdsaSha2NistP256 => {
                        unreachable!("ordinary ECDSA handled above")
                    }
                    SshSignatureAlgorithm::Unsupported(_) => {
                        unreachable!("DeviceKey rejects unsupported signature algorithms")
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::device_key::DeviceKey;
    use crate::test_support::{MemoryProfileRepository, ModelProfileFixture, SoftwareDeviceKey};
    use keyringd_ssh_agent_protocol::{SshAgentSignFlags, SshCertificateBody};
    use p256::ecdsa::{Signature, VerifyingKey, signature::Verifier};
    use sha2::{Digest, Sha256};
    use std::sync::Arc;

    keyringd_testkit::scenario! {
        group "Device Key signing application" {
            case "Signs using the application in the certificate body" {
                async: tokio;
                verify "SK and WebAuthn signatures verify for the subject application and fail for the product default" {
                    for algorithm in [SshSignatureAlgorithm::SkEcdsaSha2NistP256, SshSignatureAlgorithm::WebAuthnSkEcdsaSha2NistP256] {
                        let mut profile = ModelProfileFixture { format: algorithm.clone(), ..Default::default() }.build();
                        let original = profile.device_key();
                        let mut body = original.certificate().body().clone();
                        let SshCertificateBody::SkEcdsaP256(subject) = &mut body else { unreachable!() };
                        subject.application = b"ssh:other".to_vec();
                        let certificate = body.prepare().unwrap().with_signature(original.certificate().signature().clone());
                        let key = DeviceKey::new(original.handle().clone(), certificate, original.local_authentication(), algorithm).unwrap();
                        profile.replace_device_key(key);
                        let blob = profile.device_key().direct_identity();
                        let mut data = Vec::new();
                        let write_string = |data: &mut Vec<u8>, value: &[u8]| { data.extend_from_slice(&(value.len() as u32).to_be_bytes()); data.extend_from_slice(value); };
                        write_string(&mut data, b"session");
                        data.push(50);
                        for field in [b"alice".as_slice(), b"ssh-connection", b"publickey"] { write_string(&mut data, field); }
                        data.push(1);
                        write_string(&mut data, blob.algorithm().as_bytes());
                        write_string(&mut data, &blob.to_bytes());
                        let request = SshAgentSignRequest::new(blob, data.clone(), SshAgentSignFlags::default());
                        let store = ProfileStore::new(vec![profile.clone()], Arc::new(MemoryProfileRepository::default()));
                        let service = SoftwareDeviceKey::new(11, b"key");
                        let signature = SignWithDeviceKeyExecution::new(&service, &store).execute(&(), profile.id(), &request).await.unwrap();
                        let (ecdsa, flags, counter, client_data, extensions) = match &signature {
                            SshSignature::SkEcdsa(s) => (s.signature(), s.flags(), s.counter(), data.as_slice(), &[][..]),
                            SshSignature::WebAuthnSkEcdsa(s) => (s.signature(), s.flags(), s.counter(), s.client_data(), s.extensions()),
                            _ => panic!("expected an SK signature"),
                        };
                        let mut raw = [0; 64];
                        raw[32-ecdsa.r().len()..32].copy_from_slice(ecdsa.r());
                        raw[64-ecdsa.s().len()..].copy_from_slice(ecdsa.s());
                        let signature = Signature::from_slice(&raw).unwrap();
                        let verifier = VerifyingKey::from_sec1_bytes(&ModelProfileFixture::public_key(11)).unwrap();
                        for (application, expected) in [("ssh:other", true), ("ssh:keyringd", false)] {
                            let mut signed = Sha256::digest(application.as_bytes()).to_vec();
                            signed.push(flags);
                            signed.extend_from_slice(&counter.to_be_bytes());
                            signed.extend_from_slice(extensions);
                            signed.extend_from_slice(&Sha256::digest(client_data));
                            assert_eq!(verifier.verify(&signed, &signature).is_ok(), expected);
                        }
                    }
                }
            }
        }
    }
}
