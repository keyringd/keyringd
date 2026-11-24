use keyringd_core::model::device_key::DeviceKeyAlgorithm;
use keyringd_core::model::device_key::DevicePrivateKeyHandle;
use keyringd_core::model::device_key::DevicePublicKey;
use keyringd_core::model::device_key::LocalAuthentication;
use keyringd_core::service::DeviceKeyGenerationRequest;
use keyringd_core::service::DeviceKeyMaterial;
use keyringd_core::service::DeviceKeyProtectionChangeRequest;
use keyringd_core::service::DeviceKeyProtectionChangeResult;
use keyringd_core::service::DeviceKeyService;
use keyringd_core::service::DeviceKeySigningRequest;
use p256::ecdsa::Signature;
use security_framework::access_control::{ProtectionMode, SecAccessControl};
use security_framework::item::{ItemSearchOptions, KeyClass, Location, Reference, SearchResult};
use security_framework::key::{Algorithm, GenerateKeyOptions, KeyType, SecKey, Token};

use super::secure_enclave_device_key_error::SecureEnclaveDeviceKeyError;

const ERR_SEC_USER_CANCELED: i32 = -128;

pub(crate) struct SecureEnclaveDeviceKeyService;

impl SecureEnclaveDeviceKeyService {
    pub(crate) fn new() -> Self {
        Self
    }

    fn generate(
        &self,
        request: DeviceKeyGenerationRequest<'_>,
    ) -> Result<DeviceKeyMaterial, SecureEnclaveDeviceKeyError> {
        if !matches!(request.algorithm, DeviceKeyAlgorithm::EcdsaP256) {
            return Err(SecureEnclaveDeviceKeyError::UnsupportedAlgorithm);
        }
        let access_flags = match request.local_authentication {
            LocalAuthentication::NotRequired => {
                security_framework::passwords::AccessControlOptions::PRIVATE_KEY_USAGE
            }
            LocalAuthentication::Required => {
                security_framework::passwords::AccessControlOptions::PRIVATE_KEY_USAGE
                    | security_framework::passwords::AccessControlOptions::USER_PRESENCE
            }
        };
        let access_control = SecAccessControl::create_with_protection(
            Some(ProtectionMode::AccessibleWhenUnlockedThisDeviceOnly),
            access_flags.bits(),
        )
        .map_err(|error| self.map_error(error))?;
        let mut options = GenerateKeyOptions::default();
        options
            .set_key_type(KeyType::ec_sec_prime_random())
            .set_size_in_bits(256)
            .set_label(format!("keyringd: {}", request.profile_name))
            .set_token(Token::SecureEnclave)
            .set_location(Location::DataProtectionKeychain)
            .set_access_control(access_control);
        let private_key = SecKey::new(&options).map_err(|error| {
            if error.code() == ERR_SEC_USER_CANCELED as isize {
                SecureEnclaveDeviceKeyError::UserCancelled
            } else {
                SecureEnclaveDeviceKeyError::SecurityFramework(error.into())
            }
        })?;
        let application_label = private_key
            .application_label()
            .ok_or(SecureEnclaveDeviceKeyError::KeyNotFound)?;
        let public_key = self.public_key(&private_key)?;
        Ok(DeviceKeyMaterial {
            handle: DevicePrivateKeyHandle::new(application_label),
            public_key,
        })
    }

    fn find_private_key(
        &self,
        handle: &DevicePrivateKeyHandle,
    ) -> Result<SecKey, SecureEnclaveDeviceKeyError> {
        let result = ItemSearchOptions::new()
            .key_class(KeyClass::private())
            .application_label(handle.as_ref())
            .local_authentication_context(Some(
                super::authentication_context::authentication_context()?,
            ))
            .load_refs(true)
            .limit(1)
            .search()
            .map_err(|error| self.map_error(error))?
            .into_iter()
            .next()
            .ok_or(SecureEnclaveDeviceKeyError::KeyNotFound)?;
        match result {
            SearchResult::Ref(Reference::Key(key)) => Ok(key),
            _ => Err(SecureEnclaveDeviceKeyError::KeyNotFound),
        }
    }

    fn public_key(
        &self,
        private_key: &SecKey,
    ) -> Result<DevicePublicKey, SecureEnclaveDeviceKeyError> {
        let public = private_key
            .public_key()
            .ok_or(SecureEnclaveDeviceKeyError::MissingPublicKey)?;
        let bytes = public
            .external_representation()
            .ok_or(SecureEnclaveDeviceKeyError::MissingPublicRepresentation)?
            .to_vec();
        DevicePublicKey::from_ecdsa_p256(&bytes)
            .map_err(SecureEnclaveDeviceKeyError::InvalidPublicKey)
    }

    fn map_error(&self, error: security_framework::base::Error) -> SecureEnclaveDeviceKeyError {
        if error.code() == ERR_SEC_USER_CANCELED {
            SecureEnclaveDeviceKeyError::UserCancelled
        } else {
            SecureEnclaveDeviceKeyError::SecurityFramework(error.into())
        }
    }
}

impl DeviceKeyService for SecureEnclaveDeviceKeyService {
    type Error = SecureEnclaveDeviceKeyError;
    type OperationContext = ();

    async fn check_availability(&self) -> Result<(), Self::Error> {
        let mut options = GenerateKeyOptions::default();
        options
            .set_key_type(KeyType::ec_sec_prime_random())
            .set_size_in_bits(256)
            .set_token(Token::SecureEnclave)
            .set_access_control(
                SecAccessControl::create_with_protection(
                    Some(ProtectionMode::AccessibleWhenUnlockedThisDeviceOnly),
                    security_framework::passwords::AccessControlOptions::PRIVATE_KEY_USAGE.bits(),
                )
                .map_err(|error| self.map_error(error))?,
            );
        // No location: kSecAttrIsPermanent is false, so the probe leaves no keychain item.
        let _key = SecKey::new(&options)
            .map_err(|error| SecureEnclaveDeviceKeyError::SecurityFramework(error.into()))?;
        Ok(())
    }

    async fn generate_key(
        &self,
        _: &Self::OperationContext,
        request: DeviceKeyGenerationRequest<'_>,
    ) -> Result<DeviceKeyMaterial, Self::Error> {
        self.generate(request)
    }

    async fn change_key_protection(
        &self,
        _: &Self::OperationContext,
        request: DeviceKeyProtectionChangeRequest<'_>,
    ) -> Result<DeviceKeyProtectionChangeResult, Self::Error> {
        self.generate(DeviceKeyGenerationRequest {
            profile_name: request.profile_name,
            reason: keyringd_core::service::DeviceKeyGenerationReason::Replacement,
            algorithm: request.algorithm,
            local_authentication: request.change.requested(),
        })
        .map(|key| DeviceKeyProtectionChangeResult::replacement(key, request.handle.clone()))
    }

    async fn sign(
        &self,
        _: &Self::OperationContext,
        request: DeviceKeySigningRequest<'_>,
    ) -> Result<keyringd_core::service::DeviceKeySignature, Self::Error> {
        let key = self.find_private_key(request.handle)?;
        let local_authentication = super::key_local_authentication::key_local_authentication(&key)?;
        let digest = (request.digest)(local_authentication);
        let der = key
            .create_signature(Algorithm::ECDSASignatureDigestX962SHA256, &digest)
            .map_err(|error| {
                if error.code() == ERR_SEC_USER_CANCELED as isize {
                    SecureEnclaveDeviceKeyError::UserCancelled
                } else {
                    SecureEnclaveDeviceKeyError::SecurityFramework(error.into())
                }
            })?;
        let signature =
            Signature::from_der(&der).map_err(SecureEnclaveDeviceKeyError::InvalidSignature)?;
        Ok(keyringd_core::service::DeviceKeySignature {
            signature: signature.to_bytes().to_vec(),
            local_authentication,
        })
    }

    async fn delete_key(
        &self,
        _: &Self::OperationContext,
        handle: &DevicePrivateKeyHandle,
    ) -> Result<(), Self::Error> {
        ItemSearchOptions::new()
            .key_class(KeyClass::private())
            .application_label(handle.as_ref())
            .delete()
            .map_err(|error| self.map_error(error))
    }

    fn is_user_cancellation(&self, error: &Self::Error) -> bool {
        matches!(error, SecureEnclaveDeviceKeyError::UserCancelled)
    }
}
