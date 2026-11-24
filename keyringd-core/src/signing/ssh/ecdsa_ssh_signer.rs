use super::SshSigner;
use crate::application::ApplicationError;
use crate::model::EcdsaP256Signature;
use crate::model::device_key::DevicePrivateKeyHandle;
use crate::service::DeviceKeyService;
use crate::service::DeviceKeySigningRequest;
use keyringd_ssh_agent_protocol::SshSignature;
use sha2::{Digest, Sha256};

pub(crate) struct EcdsaSshSigner<'a, D: DeviceKeyService> {
    pub service: &'a D,
    pub context: &'a D::OperationContext,
    pub handle: &'a DevicePrivateKeyHandle,
    pub profile_name: &'a str,
}

impl<D: DeviceKeyService> SshSigner for EcdsaSshSigner<'_, D> {
    async fn sign(&mut self, message: &[u8]) -> Result<SshSignature, ApplicationError> {
        let digest: [u8; 32] = Sha256::digest(message).into();
        let raw = self
            .service
            .sign(
                self.context,
                DeviceKeySigningRequest {
                    handle: self.handle,
                    profile_name: self.profile_name,
                    digest: &|_| digest,
                },
            )
            .await
            .map_err(|error| ApplicationError::from_device_key_error(self.service, error))?;
        Ok(EcdsaP256Signature::from_raw(&raw.signature)
            .map_err(|error| ApplicationError::DeviceKey(Box::new(error)))?
            .to_ssh_signature()
            .into_signature())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::MockDeviceKeyService;
    use keyringd_ssh_agent_protocol::SshSignatureAlgorithm;

    keyringd_testkit::scenario! {
        group "EcdsaSshSigner::sign" {
            case "Signs sha256 digest with selected Device Key" {
                async: tokio;
                timeout: std::time::Duration::from_secs(10);
                verify "the selected key signs the SHA-256 digest" {
                    let mut service = MockDeviceKeyService::new();
                    let handle = DevicePrivateKeyHandle::new(b"selected-key".to_vec());
                    let expected_handle = handle.clone();
                    let message = b"original signing payload";
                    let expected_digest = Sha256::digest(message).to_vec();
                    service
                        .expect_sign()
                        .times(1)
                        .withf(move |_, request| {
                            request.handle.as_ref() == expected_handle.as_ref()
                                && request.profile_name == "Profile"
                                && (request.digest)(crate::model::device_key::LocalAuthentication::NotRequired).as_slice() == expected_digest
                        })
                        .returning(|_, _| {
                            let mut signature = vec![1; 32];
                            signature.extend_from_slice(&[2; 32]);
                            Ok(crate::service::DeviceKeySignature { signature, local_authentication: crate::model::device_key::LocalAuthentication::NotRequired })
                        });
                    let signature = EcdsaSshSigner {
                        service: &service,
                        context: &(),
                        handle: &handle,
                        profile_name: "Profile",
                    }
                    .sign(message)
                    .await
                    .unwrap();
                    assert_eq!(
                        signature.algorithm(),
                        SshSignatureAlgorithm::EcdsaSha2NistP256
                    );
                }
            }
        }
    }
}
