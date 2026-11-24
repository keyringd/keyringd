use super::CngError;
use super::CngOperationContext;
use super::CngProvider;
use super::ProviderHandle;
use super::SigningUiGuard;
use super::key_handle::KeyHandle;
use super::key_handle::export_ec_point;
use super::key_handle::sign_with_key;
use keyringd_core::model::device_key::DeviceKeyAlgorithm;
use keyringd_core::model::device_key::DevicePrivateKeyHandle;
use keyringd_core::model::device_key::DevicePublicKey;
#[cfg(test)]
use keyringd_core::model::device_key::LocalAuthentication;
use keyringd_core::service::DeviceKeyGenerationReason;
use keyringd_core::service::DeviceKeyGenerationRequest;
use keyringd_core::service::DeviceKeyMaterial;
use keyringd_core::service::DeviceKeyProtectionChangeRequest;
use keyringd_core::service::DeviceKeyProtectionChangeResult;
use keyringd_core::service::DeviceKeyService;

pub struct CngDeviceKeyService {
    pub(super) provider: CngProvider,
}

impl CngDeviceKeyService {
    pub fn new() -> Self {
        Self::platform()
    }

    pub fn platform() -> Self {
        Self {
            provider: CngProvider::Platform,
        }
    }

    #[cfg(any(test, feature = "test-support"))]
    pub fn software() -> Self {
        Self {
            provider: CngProvider::Software,
        }
    }

    pub fn delete_key(
        &self,
        context: &CngOperationContext,
        key_handle: &DevicePrivateKeyHandle,
    ) -> Result<(), CngError> {
        self.delete_named_key(context, &Self::key_name(key_handle)?)
    }
    /// Allows the fixture to record a deletion intent before creating a named key.
    pub(crate) fn generate_named_key(
        &self,
        context: &CngOperationContext,
        request: DeviceKeyGenerationRequest<'_>,
        key_name: &str,
    ) -> Result<DeviceKeyMaterial, CngError> {
        if !matches!(request.algorithm, DeviceKeyAlgorithm::EcdsaP256) {
            return Err(CngError::UnsupportedAlgorithm(request.algorithm));
        }

        let provider = ProviderHandle::open(self.provider)?;
        let key = KeyHandle::create(
            &provider,
            key_name,
            request.local_authentication,
            context.parent_window_handle(),
            request,
        )?;
        let ec_point = export_ec_point(&key)?;

        Ok(DeviceKeyMaterial {
            handle: DevicePrivateKeyHandle::new(encode_key_handle(key_name.as_bytes())),
            public_key: DevicePublicKey::from_ecdsa_p256(ec_point.as_bytes())
                .map_err(|error| CngError::InvalidDevicePublicKey(Some(Box::new(error))))?,
        })
    }

    pub(crate) fn key_name(handle: &DevicePrivateKeyHandle) -> Result<String, CngError> {
        decode_key_name(handle)
    }

    pub(crate) fn delete_named_key(
        &self,
        context: &CngOperationContext,
        name: &str,
    ) -> Result<(), CngError> {
        let provider = ProviderHandle::open(self.provider)?;
        KeyHandle::open(&provider, name, context.parent_window_handle())?.delete()
    }
}

impl Default for CngDeviceKeyService {
    fn default() -> Self {
        Self::new()
    }
}

impl DeviceKeyService for CngDeviceKeyService {
    type Error = CngError;
    type OperationContext = CngOperationContext;

    async fn check_availability(&self) -> Result<(), Self::Error> {
        let provider = ProviderHandle::open(self.provider)?;
        // An unnamed key is not persisted and is released with its handle.
        let _key = KeyHandle::create_ephemeral(&provider)?;
        Ok(())
    }

    async fn generate_key(
        &self,
        context: &CngOperationContext,
        request: DeviceKeyGenerationRequest<'_>,
    ) -> Result<DeviceKeyMaterial, Self::Error> {
        self.generate_named_key(context, request, &unique_key_name("keyringd-device-key"))
    }

    async fn change_key_protection(
        &self,
        context: &Self::OperationContext,
        request: DeviceKeyProtectionChangeRequest<'_>,
    ) -> Result<DeviceKeyProtectionChangeResult, Self::Error> {
        // CNG UI policy becomes read-only after NCryptFinalizeKey, so Windows
        // must create a replacement key when this policy changes.
        self.generate_key(
            context,
            DeviceKeyGenerationRequest {
                profile_name: request.profile_name,
                reason: DeviceKeyGenerationReason::PasswordProtectionChange,
                algorithm: request.algorithm,
                local_authentication: request.change.requested(),
            },
        )
        .await
        .map(|key| DeviceKeyProtectionChangeResult::replacement(key, request.handle.clone()))
    }

    async fn sign(
        &self,
        context: &Self::OperationContext,
        request: keyringd_core::service::DeviceKeySigningRequest<'_>,
    ) -> Result<keyringd_core::service::DeviceKeySignature, Self::Error> {
        let key_name = decode_key_name(request.handle)?;
        let provider = ProviderHandle::open(self.provider)?;
        let _ui = SigningUiGuard::show(context.parent_window_handle());
        let key = KeyHandle::open(&provider, &key_name, context.parent_window_handle())?;
        let local_authentication = key.local_authentication()?;
        let digest = (request.digest)(local_authentication);
        let signature = sign_with_key(&key, &digest)?;
        Ok(keyringd_core::service::DeviceKeySignature {
            signature,
            local_authentication,
        })
    }

    async fn delete_key(
        &self,
        context: &Self::OperationContext,
        handle: &DevicePrivateKeyHandle,
    ) -> Result<(), Self::Error> {
        CngDeviceKeyService::delete_key(self, context, handle)
    }
}

const CNG_KEY_HANDLE_MAGIC: &[u8; 8] = b"PK4TPMW1";

fn encode_key_handle(name: &[u8]) -> Vec<u8> {
    let mut buf = Vec::with_capacity(8 + name.len());
    buf.extend_from_slice(CNG_KEY_HANDLE_MAGIC);
    buf.extend_from_slice(name);
    buf
}

fn decode_key_name(key_handle: &DevicePrivateKeyHandle) -> Result<String, CngError> {
    let data = key_handle.as_ref();
    if data.len() < 8 || &data[..8] != CNG_KEY_HANDLE_MAGIC {
        return Err(CngError::InvalidKeyHandle(None));
    }
    let name =
        std::str::from_utf8(&data[8..]).map_err(|error| CngError::InvalidKeyHandle(Some(error)))?;
    if name.is_empty() || name.encode_utf16().any(|unit| unit == 0) {
        return Err(CngError::InvalidKeyHandle(None));
    }
    Ok(name.to_string())
}

fn unique_key_name(label: &str) -> String {
    let safe_label: String = label
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let mut random = [0u8; 16];
    getrandom::fill(&mut random).expect(
        "the operating system should provide randomness for the Device Key authentication salt",
    );
    let suffix = random
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    format!("keyringd-{safe_label}-{suffix}")
}

pub(super) fn wide_string(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

#[cfg(test)]
mod tests {
    use super::super::key_handle::parse_ec_public_blob;
    use super::*;
    use ecdsa::signature::hazmat::PrehashVerifier;
    use p256::ecdsa::VerifyingKey;
    use sha2::{Digest, Sha256};
    use windows::Win32::Security::Cryptography::BCRYPT_ECDSA_PUBLIC_P256_MAGIC;

    const ECC_P256_COORDINATE_SIZE: usize = 32;

    keyringd_testkit::scenario! {
        group "parse_ec_public_blob" {
            case "Can parse CNG P-256 public key blob" {
                given "Prepare a blob" {
                    let mut blob = Vec::new();
                    blob.extend_from_slice(&BCRYPT_ECDSA_PUBLIC_P256_MAGIC.to_le_bytes());
                    blob.extend_from_slice(&(ECC_P256_COORDINATE_SIZE as u32).to_le_bytes());
                    blob.extend_from_slice(&[1u8; ECC_P256_COORDINATE_SIZE]);
                    blob.extend_from_slice(&[2u8; ECC_P256_COORDINATE_SIZE]);
                }
                when "Call parse_ec_public_blob" {
                    let point = parse_ec_public_blob(&blob).unwrap();
                }
                then "Can parse CNG P-256 public key blob" {
                    assert_eq!(point.as_bytes().len(), 65);
                    assert_eq!(point.as_bytes()[0], 0x04);
                    assert_eq!(&point.as_bytes()[1..33], &[1u8; 32]);
                    assert_eq!(&point.as_bytes()[33..], &[2u8; 32]);
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "decode_key_name" {
            case "Reject an incorrect CNG key name" {
                verify "Reject an incorrect CNG key name" {
                    assert!(decode_key_name(&DevicePrivateKeyHandle::new(Vec::new())).is_err());
                    assert!(decode_key_name(&DevicePrivateKeyHandle::new(vec![0xff])).is_err());
                    assert!(decode_key_name(&DevicePrivateKeyHandle::new(b"bad\0name".to_vec())).is_err());
                    assert!(decode_key_name(&DevicePrivateKeyHandle::new(b"no-magic".to_vec())).is_err());
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "CngDeviceKeyService::sign" {
            case "Signs with a software KSP key and verifies the signature" {
                given "Prepare a Software KSP Provider and Temporary ECDSA Key" {
                    let store = CngDeviceKeyService::software();
                    let provider = ProviderHandle::open(store.provider).unwrap();
                    let key = KeyHandle::create_ephemeral(&provider).unwrap();
                }
                when "Sign with the key created by exporting the public key" {
                    let ec_point = export_ec_point(&key).unwrap();
                }
                then "You can validate a signature with an exported public key" {
                    assert_eq!(key.local_authentication().unwrap(), LocalAuthentication::NotRequired);
                    verify_signature(&key, ec_point.as_bytes());
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "CngDeviceKeyService::sign" {
            case "Signs with a TPM key and verifies the signature" {
                async: tokio;
                timeout: std::time::Duration::from_secs(30);
                ignored: "requires a Windows TPM and creates a temporary persisted key";
                verify "You can back-and-trip signatures with TPM keys" {
                    key_round_trip(
                        CngDeviceKeyService::platform(),
                        LocalAuthentication::NotRequired,
                    )
                    .await;
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "CngDeviceKeyService::sign" {
            case "Round-trip signatures can be verified with a TPM key that requires strong protection" {
                async: tokio;
                timeout: std::time::Duration::from_secs(120);
                ignored: "interactive: creates a TPM key and opens Windows strong-key-protection UI";
                verify "Round-trip signatures can be verified with a TPM key that requires strong protection" {
                    key_round_trip(
                        CngDeviceKeyService::platform(),
                        LocalAuthentication::Required,
                    )
                    .await;
                }
            }
        }
    }

    async fn key_round_trip(store: CngDeviceKeyService, protection: LocalAuthentication) {
        let context = CngOperationContext::default();
        let DeviceKeyMaterial {
            handle,
            public_key: pubkey,
            ..
        } = store
            .generate_key(
                &context,
                DeviceKeyGenerationRequest {
                    profile_name: "Test",
                    reason: keyringd_core::service::DeviceKeyGenerationReason::InitialSetup,
                    algorithm: DeviceKeyAlgorithm::EcdsaP256,
                    local_authentication: protection,
                },
            )
            .await
            .unwrap();
        let key_name = decode_key_name(&handle).unwrap();

        let provider = ProviderHandle::open(store.provider).unwrap();
        let opened_key = KeyHandle::open(&provider, &key_name, 0).unwrap();
        assert_eq!(opened_key.local_authentication().unwrap(), protection);
        verify_signature(&opened_key, pubkey.public_key_bytes());
        let digest = [42; 32];
        let signed = DeviceKeyService::sign(
            &store,
            &context,
            keyringd_core::service::DeviceKeySigningRequest {
                handle: &handle,
                profile_name: "Test",
                digest: &|authentication| {
                    assert_eq!(authentication, protection);
                    digest
                },
            },
        )
        .await
        .unwrap();
        assert_eq!(signed.local_authentication, protection);
        let signature = p256::ecdsa::Signature::from_slice(&signed.signature).unwrap();
        VerifyingKey::from_sec1_bytes(pubkey.public_key_bytes())
            .unwrap()
            .verify_prehash(&digest, &signature)
            .unwrap();
        opened_key.delete().unwrap();
    }

    fn verify_signature(key: &KeyHandle, public_key_bytes: &[u8]) {
        let digest = Sha256::digest(b"keyringd CNG provider test");
        let sig_bytes = sign_with_key(key, &digest).unwrap();
        let signature = p256::ecdsa::Signature::from_slice(&sig_bytes).unwrap();
        let verifying_key = VerifyingKey::from_sec1_bytes(public_key_bytes).unwrap();
        verifying_key.verify_prehash(&digest, &signature).unwrap();
    }
}
