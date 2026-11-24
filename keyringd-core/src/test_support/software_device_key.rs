use super::SoftwareDeviceKeyError;
use keyringd_core::model::device_key::DeviceKeyAlgorithm;
use keyringd_core::model::device_key::DevicePrivateKeyHandle;
use keyringd_core::model::device_key::DevicePublicKey;
use keyringd_core::service::DeviceKeyGenerationReason;
use keyringd_core::service::DeviceKeyGenerationRequest;
use keyringd_core::service::DeviceKeyMaterial;
use keyringd_core::service::DeviceKeyProtectionChangeRequest;
use keyringd_core::service::DeviceKeyProtectionChangeResult;
use keyringd_core::service::DeviceKeyService;
use p256::ecdsa::signature::hazmat::PrehashSigner;
use p256::ecdsa::{Signature, SigningKey};
use sha2::{Digest, Sha256};
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};
#[derive(Clone)]
pub struct SoftwareDeviceKey {
    signing_key: Arc<SigningKey>,
    handle: &'static [u8],
    generations: Option<Arc<AtomicU64>>,
    available: bool,
}
impl SoftwareDeviceKey {
    pub fn new(seed: u8, handle: &'static [u8]) -> Self {
        Self {
            signing_key: Arc::new(SigningKey::from_slice(&[seed; 32]).unwrap()),
            handle,
            generations: None,
            available: true,
        }
    }
    /// Simulates hardware that fails the availability check.
    pub fn unavailable() -> Self {
        Self {
            available: false,
            ..Self::generating()
        }
    }

    /// Generates independent software keys and reconstructs them from their test
    /// handles after restarting the fixture. Handles contain no production keys.
    pub fn generating() -> Self {
        let mut service = Self::new(11, b"windows-ui-test-key");
        let epoch = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;
        service.generations = Some(Arc::new(AtomicU64::new(epoch)));
        service
    }
    fn key_for_handle(
        &self,
        handle: &DevicePrivateKeyHandle,
    ) -> Result<SigningKey, SoftwareDeviceKeyError> {
        if self.generations.is_none() {
            return Ok((*self.signing_key).clone());
        }
        SigningKey::from_slice(&Sha256::digest(handle.as_ref())).map_err(|_| SoftwareDeviceKeyError)
    }
    fn public_key(signing_key: &SigningKey) -> DevicePublicKey {
        DevicePublicKey::from_ecdsa_p256(
            signing_key.verifying_key().to_sec1_point(false).as_bytes(),
        )
        .unwrap()
    }
}
impl DeviceKeyService for SoftwareDeviceKey {
    type Error = SoftwareDeviceKeyError;
    type OperationContext = ();
    async fn check_availability(&self) -> Result<(), Self::Error> {
        if self.available {
            Ok(())
        } else {
            Err(SoftwareDeviceKeyError)
        }
    }
    async fn generate_key(
        &self,
        _: &Self::OperationContext,
        request: DeviceKeyGenerationRequest<'_>,
    ) -> Result<DeviceKeyMaterial, Self::Error> {
        if !matches!(request.algorithm, DeviceKeyAlgorithm::EcdsaP256) {
            return Err(SoftwareDeviceKeyError);
        }
        let mut handle = self.handle.to_vec();
        if let Some(generations) = &self.generations {
            handle.extend_from_slice(&generations.fetch_add(1, Ordering::SeqCst).to_be_bytes());
            // Persist the simulated authentication policy across fixture restarts.
            handle.push(match request.local_authentication {
                keyringd_core::model::device_key::LocalAuthentication::Required => 0x05,
                keyringd_core::model::device_key::LocalAuthentication::NotRequired => 0,
            });
        }
        let handle = DevicePrivateKeyHandle::new(handle);
        let key = self.key_for_handle(&handle)?;
        Ok(DeviceKeyMaterial {
            handle,
            public_key: Self::public_key(&key),
        })
    }

    async fn change_key_protection(
        &self,
        _: &Self::OperationContext,
        request: DeviceKeyProtectionChangeRequest<'_>,
    ) -> Result<DeviceKeyProtectionChangeResult, Self::Error> {
        if !matches!(request.algorithm, DeviceKeyAlgorithm::EcdsaP256) {
            return Err(SoftwareDeviceKeyError);
        }
        if self.generations.is_some() {
            // CNG changes protection by generating a replacement key. Keep the
            // fixed-key constructor's behavior for existing application tests.
            return self
                .generate_key(
                    &(),
                    DeviceKeyGenerationRequest {
                        reason: DeviceKeyGenerationReason::PasswordProtectionChange,
                        algorithm: request.algorithm,
                        profile_name: request.profile_name,
                        local_authentication: request.change.requested(),
                    },
                )
                .await
                .map(|key| {
                    DeviceKeyProtectionChangeResult::replacement(key, request.handle.clone())
                });
        }
        Ok(DeviceKeyProtectionChangeResult::updated(
            DeviceKeyMaterial {
                handle: request.handle.clone(),
                public_key: Self::public_key(&self.key_for_handle(request.handle)?),
            },
        ))
    }
    async fn sign(
        &self,
        _: &Self::OperationContext,
        request: keyringd_core::service::DeviceKeySigningRequest<'_>,
    ) -> Result<keyringd_core::service::DeviceKeySignature, Self::Error> {
        let policy = if self.generations.is_some() {
            *request
                .handle
                .as_ref()
                .last()
                .ok_or(SoftwareDeviceKeyError)?
        } else {
            0
        };
        let local_authentication = match policy {
            0 => keyringd_core::model::device_key::LocalAuthentication::NotRequired,
            5 => keyringd_core::model::device_key::LocalAuthentication::Required,
            _ => return Err(SoftwareDeviceKeyError),
        };
        let signature: Signature = self
            .key_for_handle(request.handle)?
            .sign_prehash(&(request.digest)(local_authentication))
            .map_err(|_| SoftwareDeviceKeyError)?;
        Ok(keyringd_core::service::DeviceKeySignature {
            signature: signature.to_bytes().to_vec(),
            local_authentication,
        })
    }

    async fn delete_key(
        &self,
        _: &Self::OperationContext,
        _: &DevicePrivateKeyHandle,
    ) -> Result<(), Self::Error> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use keyringd_core::model::device_key::LocalAuthentication;
    use keyringd_core::service::DeviceKeyGenerationReason;
    use keyringd_core::service::DeviceKeySigningRequest;
    use p256::ecdsa::VerifyingKey;
    use p256::ecdsa::signature::hazmat::PrehashVerifier;

    keyringd_testkit::scenario! {
        group "SoftwareDeviceKey::sign" {
            case "Generated fixture preserves local authentication requirements after restart" {
                async: tokio;
                timeout: std::time::Duration::from_secs(10);
                verify "restore the generated key with the same local authentication policy" {
                    for policy in [LocalAuthentication::NotRequired, LocalAuthentication::Required] {
                        let service = SoftwareDeviceKey::generating();
                        let DeviceKeyMaterial {
                            handle,
                            public_key: public,
                            ..
                        } = service
                            .generate_key(
                                &(),
                                DeviceKeyGenerationRequest {
                                    reason: DeviceKeyGenerationReason::InitialSetup,
                                    algorithm: DeviceKeyAlgorithm::EcdsaP256,
                                    profile_name: "test",
                                    local_authentication: policy,
                                },
                            )
                            .await
                            .unwrap();
                        let restored = SoftwareDeviceKey::generating();
                        let digest = [42; 32];
                        let signed = restored.sign(&(), DeviceKeySigningRequest {
                            handle: &handle,
                            profile_name: "test",
                            digest: &|authentication| {
                                assert_eq!(authentication, policy);
                                digest
                            },
                        }).await.unwrap();
                        assert_eq!(signed.local_authentication, policy);
                        VerifyingKey::from_sec1_bytes(public.public_key_bytes())
                            .unwrap()
                            .verify_prehash(
                                &digest,
                                &Signature::from_slice(&signed.signature).unwrap(),
                            )
                            .unwrap();
                    }
                }
            }
        }
        group "SoftwareDeviceKey::sign" {
            case "Generates independent software Device Keys and signs from a handle in a new service" {
                async: tokio;
                given "Software service that generates independent keys" {
                    let service = SoftwareDeviceKey::generating();
                    let request = || DeviceKeyGenerationRequest {
                        reason: DeviceKeyGenerationReason::InitialSetup,
                        algorithm: DeviceKeyAlgorithm::EcdsaP256,
                        profile_name: "test",
                        local_authentication: LocalAuthentication::NotRequired,
                    };
                }
                when "Generate two keys to renew the service" {
                    let DeviceKeyMaterial {
                        handle: first,
                        public_key: first_public,
                        ..
                    } = service.generate_key(&(), request()).await.unwrap();
                    let DeviceKeyMaterial {
                        handle: _,
                        public_key: second_public,
                        ..
                    } = service.generate_key(&(), request()).await.unwrap();
                    let restored = SoftwareDeviceKey::generating();
                    let digest = [42u8; 32];
                    let signature = restored
                        .sign(
                            &(),
                            DeviceKeySigningRequest {
                                handle: &first,
                                profile_name: "test",
                                digest: &|_| digest,
                            },
                        )
                        .await
                        .unwrap();
                }
                then "Different public keys can verify the signature of the rebuilt key with the original public key" {
                    assert_ne!(
                        first_public.public_key_bytes(),
                        second_public.public_key_bytes()
                    );
                    let verifier = VerifyingKey::from_sec1_bytes(first_public.public_key_bytes()).unwrap();
                    verifier
                        .verify_prehash(&digest, &Signature::from_slice(&signature.signature).unwrap())
                        .unwrap();
                }
            }
        }
    }
}
