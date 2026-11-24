use crate::app::service::DeviceKeyInteraction;
use keyringd_core::service::DeviceKeyMaterial;
use keyringd_core::service::DeviceKeyProtectionChangeResult;
use keyringd_ui_core::key_operation::DeviceKeyPasswordChangeInput;
use p256::Sec1Point;
use std::sync::Arc;
use tss_esapi::{
    Context, TctiNameConf,
    attributes::ObjectAttributesBuilder,
    interface_types::{
        algorithm::{EccSchemeAlgorithm, HashingAlgorithm, PublicAlgorithm},
        ecc::EccCurve,
        key_bits::AesKeyBits,
        resource_handles::Hierarchy,
        session_handles::AuthSession,
    },
    structures::{
        Auth, Digest as TpmDigest, EccPoint, EccScheme, KeyDerivationFunctionScheme, Public,
        PublicBuilder, PublicEccParametersBuilder, Signature as TpmSignature, SignatureScheme,
        SymmetricDefinitionObject,
    },
};
use zeroize::Zeroizing;

use super::EsapiError;
use super::key_handle_data::{AUTH_SALT_LENGTH, KeyHandleData};
use super::tpm_access::{RESOURCE_MANAGER, check_resource_manager_access};
use keyringd_core::model::device_key::DeviceKeyAlgorithm;
use keyringd_core::model::device_key::DevicePrivateKeyHandle;
use keyringd_core::model::device_key::DevicePublicKey;
use keyringd_core::model::device_key::LocalAuthentication;
use keyringd_core::service::DeviceKeyGenerationRequest;
use keyringd_core::service::DeviceKeyProtectionChangeRequest;
use keyringd_core::service::DeviceKeyService;

pub(crate) struct EsapiDeviceKeyService {
    tcti: TctiNameConf,
    interaction: Option<Arc<dyn DeviceKeyInteraction>>,
    uses_resource_manager: bool,
}

impl EsapiDeviceKeyService {
    pub fn platform(interaction: Option<Arc<dyn DeviceKeyInteraction>>) -> Self {
        let has_override = tcti_environment_is_configured();
        let uses_resource_manager =
            !has_override && std::path::Path::new(RESOURCE_MANAGER).exists();
        let tcti = if has_override {
            TctiNameConf::from_environment_variable().expect(
                "the configured TPM TCTI should use the supported TCTI configuration syntax",
            )
        } else if uses_resource_manager {
            "device:/dev/tpmrm0".parse().expect(
                "the built-in device:/dev/tpmrm0 literal should be a valid TCTI configuration",
            )
        } else {
            TctiNameConf::Device(Default::default())
        };
        Self {
            uses_resource_manager,
            ..Self::new(tcti, interaction)
        }
    }

    pub fn new(tcti: TctiNameConf, interaction: Option<Arc<dyn DeviceKeyInteraction>>) -> Self {
        Self {
            tcti,
            interaction,
            uses_resource_manager: false,
        }
    }

    fn create_context(&self) -> Result<Context, EsapiError> {
        let mut ctx = Context::new(self.tcti.clone())?;
        // TPM may already be initialized (e.g., by another process).
        // Ignore TPM2_RC_INITIALIZE; propagate other errors.
        match ctx.startup(tss_esapi::constants::StartupType::Clear) {
            Ok(()) => {}
            Err(tss_esapi::Error::Tss2Error(rc))
                if rc == tss_esapi::constants::tss::TPM2_RC_INITIALIZE.into() => {}
            Err(error) => return Err(error.into()),
        }
        Ok(ctx)
    }

    fn create_primary_key(
        &self,
        ctx: &mut Context,
    ) -> Result<tss_esapi::handles::KeyHandle, EsapiError> {
        let primary_public = PublicBuilder::new()
            .with_public_algorithm(PublicAlgorithm::Ecc)
            .with_name_hashing_algorithm(HashingAlgorithm::Sha256)
            .with_object_attributes(
                ObjectAttributesBuilder::new()
                    .with_fixed_tpm(true)
                    .with_fixed_parent(true)
                    .with_sensitive_data_origin(true)
                    .with_user_with_auth(true)
                    .with_decrypt(true)
                    .with_restricted(true)
                    .build()?,
            )
            .with_ecc_parameters(
                PublicEccParametersBuilder::new()
                    .with_ecc_scheme(EccScheme::Null)
                    .with_curve(EccCurve::NistP256)
                    .with_is_signing_key(false)
                    .with_is_decryption_key(true)
                    .with_restricted(true)
                    .with_key_derivation_function_scheme(KeyDerivationFunctionScheme::Null)
                    .with_symmetric(SymmetricDefinitionObject::Aes {
                        key_bits: AesKeyBits::Aes128,
                        mode: tss_esapi::interface_types::algorithm::SymmetricMode::Cfb,
                    })
                    .build()?,
            )
            .with_ecc_unique_identifier(EccPoint::default())
            .build()?;

        let result =
            ctx.create_primary(Hierarchy::Owner, primary_public, None, None, None, None)?;
        Ok(result.key_handle)
    }
}

fn tcti_environment_is_configured() -> bool {
    ["TPM2TOOLS_TCTI", "TCTI", "TEST_TCTI"]
        .into_iter()
        .any(|name| std::env::var_os(name).is_some())
}

const EC_UNCOMPRESSED_PREFIX: u8 = 0x04;
const P256_COORD_LEN: usize = 32;
const P256_UNCOMPRESSED_POINT_LEN: usize = 1 + P256_COORD_LEN * 2;
const P256_RAW_SIG_LEN: usize = P256_COORD_LEN * 2;

fn extract_ec_point(public: &Public) -> Result<Sec1Point, EsapiError> {
    match public {
        Public::Ecc { unique, .. } => {
            let x = unique.x().as_slice();
            let y = unique.y().as_slice();
            let mut uncompressed = [0u8; P256_UNCOMPRESSED_POINT_LEN];
            uncompressed[0] = EC_UNCOMPRESSED_PREFIX;
            uncompressed[1 + (P256_COORD_LEN - x.len())..1 + P256_COORD_LEN].copy_from_slice(x);
            uncompressed
                [1 + P256_COORD_LEN + (P256_COORD_LEN - y.len())..P256_UNCOMPRESSED_POINT_LEN]
                .copy_from_slice(y);
            Sec1Point::from_bytes(uncompressed)
                .map_err(|error| EsapiError::InvalidEcPoint(Some(error.into())))
        }
        _ => Err(EsapiError::InvalidEcPoint(None)),
    }
}

impl EsapiDeviceKeyService {
    fn generate_key_sync(
        &self,
        algorithm: DeviceKeyAlgorithm,
        password: Option<Zeroizing<String>>,
    ) -> Result<DeviceKeyMaterial, EsapiError> {
        if !matches!(algorithm, DeviceKeyAlgorithm::EcdsaP256) {
            return Err(EsapiError::UnsupportedAlgorithm(algorithm));
        }

        let mut ctx = self.create_context()?;
        ctx.set_sessions((Some(AuthSession::Password), None, None));
        let primary_handle = self.create_primary_key(&mut ctx)?;

        let signing_public = PublicBuilder::new()
            .with_public_algorithm(PublicAlgorithm::Ecc)
            .with_name_hashing_algorithm(HashingAlgorithm::Sha256)
            .with_object_attributes(
                ObjectAttributesBuilder::new()
                    .with_fixed_tpm(true)
                    .with_fixed_parent(true)
                    .with_sensitive_data_origin(true)
                    .with_user_with_auth(true)
                    .with_sign_encrypt(true)
                    .build()?,
            )
            .with_ecc_parameters(
                PublicEccParametersBuilder::new()
                    .with_ecc_scheme(EccScheme::create(
                        EccSchemeAlgorithm::EcDsa,
                        Some(HashingAlgorithm::Sha256),
                        None,
                    )?)
                    .with_curve(EccCurve::NistP256)
                    .with_is_signing_key(true)
                    .with_is_decryption_key(false)
                    .with_restricted(false)
                    .with_key_derivation_function_scheme(KeyDerivationFunctionScheme::Null)
                    .build()?,
            )
            .with_ecc_unique_identifier(EccPoint::default())
            .build()?;

        let auth_salt = password.as_ref().map(|_| {
            let mut salt = [0; AUTH_SALT_LENGTH];
            getrandom::fill(&mut salt)
                .expect("the operating system should provide randomness for the Device Key authentication salt");
            salt
        });
        let auth_value = match (&password, &auth_salt) {
            (Some(password), Some(salt)) => Some(derive_auth_value(password, salt)?),
            _ => None,
        };
        let result = ctx.create(primary_handle, signing_public, auth_value, None, None, None)?;
        let ec_point = extract_ec_point(&result.out_public)?;

        let key_handle_data = KeyHandleData {
            private: result.out_private,
            public: result.out_public,
            auth_salt,
        };

        let handle = DevicePrivateKeyHandle::new(key_handle_data.serialize()?);
        let public_key = DevicePublicKey::from_ecdsa_p256(ec_point.as_bytes())
            .map_err(|error| EsapiError::InvalidEcPoint(Some(Box::new(error))))?;
        Ok(DeviceKeyMaterial { handle, public_key })
    }

    fn sign_sync(
        &self,
        key_data: KeyHandleData,
        data: &[u8],
        password: Option<Zeroizing<String>>,
    ) -> Result<Vec<u8>, EsapiError> {
        let mut ctx = self.create_context()?;
        ctx.set_sessions((Some(AuthSession::Password), None, None));
        let primary_handle = self.create_primary_key(&mut ctx)?;
        let loaded = ctx.load(primary_handle, key_data.private, key_data.public)?;
        if let Some(salt) = key_data.auth_salt {
            let password = password.ok_or(EsapiError::PasswordCancelled)?;
            ctx.tr_set_auth(loaded.into(), derive_auth_value(&password, &salt)?)?;
        }

        let digest =
            TpmDigest::try_from(data.to_vec()).map_err(|_| EsapiError::InvalidDigestLength)?;

        let signature = ctx.sign(
            loaded,
            digest,
            SignatureScheme::Null,
            tss_esapi::structures::HashcheckTicket::try_from(
                tss_esapi::tss2_esys::TPMT_TK_HASHCHECK {
                    tag: tss_esapi::constants::tss::TPM2_ST_HASHCHECK,
                    hierarchy: tss_esapi::constants::tss::TPM2_RH_NULL,
                    digest: Default::default(),
                },
            )?,
        )?;

        match signature {
            TpmSignature::EcDsa(sig) => {
                let r = sig.signature_r().as_slice();
                let s = sig.signature_s().as_slice();
                let mut fixed = [0u8; P256_RAW_SIG_LEN];
                fixed[P256_COORD_LEN - r.len()..P256_COORD_LEN].copy_from_slice(r);
                fixed[P256_RAW_SIG_LEN - s.len()..P256_RAW_SIG_LEN].copy_from_slice(s);
                Ok(fixed.to_vec())
            }
            _ => Err(EsapiError::InvalidKeyHandle(
                "unexpected TPM signature type".into(),
            )),
        }
    }

    fn change_key_protection_sync(
        &self,
        key_handle: &DevicePrivateKeyHandle,
        current_password: Option<Zeroizing<String>>,
        new_password: Option<Zeroizing<String>>,
    ) -> Result<DeviceKeyMaterial, EsapiError> {
        let key_data = KeyHandleData::deserialize(key_handle.as_ref())
            .map_err(|error| EsapiError::InvalidKeyHandle(Box::new(error)))?;
        let public_key = extract_ec_point(&key_data.public)?;
        let mut ctx = self.create_context()?;
        ctx.set_sessions((Some(AuthSession::Password), None, None));
        let primary_handle = self.create_primary_key(&mut ctx)?;
        let loaded = ctx.load(primary_handle, key_data.private, key_data.public.clone())?;
        if let Some(salt) = key_data.auth_salt {
            let password = current_password.ok_or(EsapiError::PasswordCancelled)?;
            ctx.tr_set_auth(loaded.into(), derive_auth_value(&password, &salt)?)?;
        }
        let new_salt = new_password.as_ref().map(|_| {
            let mut salt = [0; AUTH_SALT_LENGTH];
            getrandom::fill(&mut salt)
                .expect("the operating system should provide randomness for the Device Key authentication salt");
            salt
        });
        let new_auth = match (&new_password, &new_salt) {
            (Some(password), Some(salt)) => derive_auth_value(password, salt)?,
            _ => Auth::default(),
        };
        let private = ctx.object_change_auth(loaded.into(), primary_handle.into(), new_auth)?;
        let updated = KeyHandleData {
            private,
            public: key_data.public,
            auth_salt: new_salt,
        };
        Ok(DeviceKeyMaterial {
            handle: DevicePrivateKeyHandle::new(updated.serialize()?),
            public_key: DevicePublicKey::from_ecdsa_p256(public_key.as_bytes())
                .map_err(|error| EsapiError::InvalidEcPoint(Some(Box::new(error))))?,
        })
    }
}

impl DeviceKeyService for EsapiDeviceKeyService {
    type Error = EsapiError;
    type OperationContext = ();

    async fn check_availability(&self) -> Result<(), Self::Error> {
        let tcti = self.tcti.clone();
        let uses_resource_manager = self.uses_resource_manager;
        tokio::task::spawn_blocking(move || {
            let result = Self::new(tcti, None).create_context().map(drop);
            if result.is_err()
                && uses_resource_manager
                && let Err(error) = check_resource_manager_access()
                && error.kind() == std::io::ErrorKind::PermissionDenied
            {
                return Err(EsapiError::AccessDenied(error));
            }
            result
        })
        .await?
    }

    async fn generate_key(
        &self,
        _: &Self::OperationContext,
        request: DeviceKeyGenerationRequest<'_>,
    ) -> Result<DeviceKeyMaterial, Self::Error> {
        let password = if request.local_authentication == LocalAuthentication::Required {
            Some(Zeroizing::new(
                self.interaction
                    .as_ref()
                    .ok_or_else(missing_interaction_error)?
                    .request_new_password(request.profile_name)
                    .await?
                    .ok_or(EsapiError::PasswordCancelled)?,
            ))
        } else {
            None
        };
        let tcti = self.tcti.clone();
        tokio::task::spawn_blocking({
            let span = tracing::Span::current();
            move || {
                span.in_scope(|| {
                    EsapiDeviceKeyService::new(tcti, None)
                        .generate_key_sync(request.algorithm, password)
                })
            }
        })
        .await?
    }

    async fn change_key_protection(
        &self,
        _: &Self::OperationContext,
        request: DeviceKeyProtectionChangeRequest<'_>,
    ) -> Result<DeviceKeyProtectionChangeResult, Self::Error> {
        let change = self
            .interaction
            .as_ref()
            .ok_or_else(missing_interaction_error)?
            .request_password_change(request.profile_name, request.change)
            .await?
            .ok_or(EsapiError::PasswordCancelled)?;
        let (current_password, new_password) = match change {
            DeviceKeyPasswordChangeInput::Enable { new_password } => {
                (None, Some(Zeroizing::new(new_password)))
            }
            DeviceKeyPasswordChangeInput::Change {
                current_password,
                new_password,
            } => (
                Some(Zeroizing::new(current_password)),
                Some(Zeroizing::new(new_password)),
            ),
            DeviceKeyPasswordChangeInput::Disable { current_password } => {
                (Some(Zeroizing::new(current_password)), None)
            }
        };
        let tcti = self.tcti.clone();
        let handle = request.handle.clone();
        tokio::task::spawn_blocking({
            let span = tracing::Span::current();
            move || {
                span.in_scope(|| {
                    EsapiDeviceKeyService::new(tcti, None).change_key_protection_sync(
                        &handle,
                        current_password,
                        new_password,
                    )
                })
            }
        })
        .await?
        .map(DeviceKeyProtectionChangeResult::updated)
    }

    async fn sign(
        &self,
        _: &Self::OperationContext,
        request: keyringd_core::service::DeviceKeySigningRequest<'_>,
    ) -> Result<keyringd_core::service::DeviceKeySignature, Self::Error> {
        let key_data = KeyHandleData::deserialize(request.handle.as_ref())
            .map_err(|error| EsapiError::InvalidKeyHandle(Box::new(error)))?;
        let password = if key_data.auth_salt.is_some() {
            Some(Zeroizing::new(
                self.interaction
                    .as_ref()
                    .ok_or_else(missing_interaction_error)?
                    .request_password(request.profile_name)
                    .await?
                    .ok_or(EsapiError::PasswordCancelled)?,
            ))
        } else {
            None
        };
        let tcti = self.tcti.clone();
        let local_authentication = if password.is_some() {
            keyringd_core::model::device_key::LocalAuthentication::Required
        } else {
            keyringd_core::model::device_key::LocalAuthentication::NotRequired
        };
        let data = (request.digest)(local_authentication);
        let signature = tokio::task::spawn_blocking({
            let span = tracing::Span::current();
            move || {
                span.in_scope(|| {
                    EsapiDeviceKeyService::new(tcti, None).sign_sync(key_data, &data, password)
                })
            }
        })
        .await??;
        Ok(keyringd_core::service::DeviceKeySignature {
            signature,
            local_authentication,
        })
    }

    async fn delete_key(
        &self,
        _: &Self::OperationContext,
        _handle: &DevicePrivateKeyHandle,
    ) -> Result<(), Self::Error> {
        // The handle contains reloadable TPM private/public blobs, not a persistent TPM object.
        // Removing the blob from the profile store is therefore the complete deletion operation.
        Ok(())
    }

    fn is_user_cancellation(&self, error: &Self::Error) -> bool {
        matches!(error, EsapiError::PasswordCancelled)
    }
}

fn missing_interaction_error() -> std::io::Error {
    std::io::Error::new(
        std::io::ErrorKind::NotConnected,
        "TPM password interaction is unavailable",
    )
}

fn derive_auth_value(password: &str, salt: &[u8; AUTH_SALT_LENGTH]) -> Result<Auth, EsapiError> {
    let mut value = zeroize::Zeroizing::new(vec![0; 32]);
    argon2::Argon2::default()
        .hash_password_into(password.as_bytes(), salt, &mut value)
        .map_err(|_| EsapiError::PasswordDerivation)?;
    Auth::try_from(value.to_vec()).map_err(EsapiError::Tss)
}

#[cfg(all(test, feature = "test-support"))]
mod tests {
    use super::*;
    use crate::system::device_key::test_support::*;

    fn signing_key(handle: &DevicePrivateKeyHandle) -> KeyHandleData {
        KeyHandleData::deserialize(handle.as_ref()).unwrap()
    }

    keyringd_testkit::scenario! {
        group "EsapiDeviceKeyService::is_user_cancellation" {
            case "Categorize the CANCEL of the PASSWORD input as a USER CANCEL" {
                verify "Confirm that the CANCEL of the PASSWORD input is classified as a User Cancel" {
                    let service = EsapiDeviceKeyService::platform(None);
                    assert!(service.is_user_cancellation(&EsapiError::PasswordCancelled));
                    assert!(!service.is_user_cancellation(&EsapiError::PasswordDerivation));
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "EsapiDeviceKeyService::generate_key_sync" {
            case "Key generation returns a valid public key" {
                given "Prepare the available Software TPM" {
                    let _lock = TPM_LOCK.lock().unwrap_or_else(|e| e.into_inner());
                    let store = require_swtpm();
                }
                when "Generate ECDSA P-256 DEVICE KEY" {
                    let DeviceKeyMaterial {
                        handle,
                        public_key: pubkey,
                        ..
                    } = store
                        .generate_key_sync(DeviceKeyAlgorithm::EcdsaP256, None)
                        .unwrap();
                }
                then "Return non-empty handle and uncompressed public key" {
                    assert_eq!(pubkey.public_key_bytes().len(), 65);
                    assert_eq!(pubkey.public_key_bytes()[0], 0x04);
                    assert!(!handle.as_ref().is_empty());
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "EsapiDeviceKeyService::generate_key_sync" {
            case "ED25519 Key generation request returns ERROR" {
                given "Prepare a Software TPM Service that only handles ECDSA Device Key" {
                    let _lock = TPM_LOCK.lock().unwrap_or_else(|e| e.into_inner());
                    let store = require_swtpm();
                }
                when "Requires unsupported ED25519 Key generation" {
                    let result = store.generate_key_sync(DeviceKeyAlgorithm::Ed25519, None);
                }
                then "Returns an ERROR without generating a key" {
                    assert!(result.is_err());
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "EsapiDeviceKeyService::sign_sync" {
            case "Sign with generated key" {
                given "Generate ECDSA Device Key with Software TPM" {
                    let _lock = TPM_LOCK.lock().unwrap_or_else(|e| e.into_inner());
                    let store = require_swtpm();
                    let DeviceKeyMaterial {
                        handle,
                        public_key: _pubkey,
                        ..
                    } = store
                        .generate_key_sync(DeviceKeyAlgorithm::EcdsaP256, None)
                        .unwrap();
                }
                when "Sign the Message Digest with the generated key" {
                    let sig_bytes = store
                        .sign_sync(signing_key(&handle), &sha2_digest(b"test message"), None)
                        .unwrap();
                }
                then "Returns RAW Signature for P-256" {
                    assert_eq!(sig_bytes.len(), 64);
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "EsapiDeviceKeyService::sign_sync" {
            case "Generate and validate ECDSA signatures" {
                given "Capture the type that prehash validates the generated ECDSA signature" {
                    use ecdsa::signature::hazmat::PrehashVerifier;
                    use p256::ecdsa::VerifyingKey;
                }
                when "Generate a P-256 Key with TPM and Sign ECDSA to Digest" {
                    let _lock = TPM_LOCK.lock().unwrap_or_else(|e| e.into_inner());
                    let store = require_swtpm();
                    let DeviceKeyMaterial {
                        handle,
                        public_key: pubkey,
                        ..
                    } = store
                        .generate_key_sync(DeviceKeyAlgorithm::EcdsaP256, None)
                        .unwrap();
                    let digest = sha2_digest(b"hello world");
                    let sig_bytes = store.sign_sync(signing_key(&handle), &digest, None).unwrap();
                }
                then "Generate and validate ECDSA signatures" {
                    let verifying_key = VerifyingKey::from_sec1_bytes(pubkey.public_key_bytes()).unwrap();
                    let p256_sig = p256::ecdsa::Signature::from_slice(&sig_bytes).unwrap();
                    verifying_key
                        .verify_prehash(&digest, &p256_sig)
                        .expect("the signature produced by the test Device Key should verify with its public key");
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "EsapiDeviceKeyService::sign_sync" {
            case "Password protection key rejects the wrong Password" {
                given "Generate a PASSWORD protected Device Key" {
                    let _lock = TPM_LOCK.lock().unwrap_or_else(|e| e.into_inner());
                    let store = require_swtpm();
                    let password = "correct horse battery staple";
                    let DeviceKeyMaterial { handle, .. } = store
                        .generate_key_sync(
                            DeviceKeyAlgorithm::EcdsaP256,
                            Some(Zeroizing::new(password.to_owned())),
                        )
                        .unwrap();
                    let digest = sha2_digest(b"protected message");
                }
                when "Sign each with the correct Password and the wrong Password" {
                    let correct = store.sign_sync(signing_key(&handle), &digest, Some(Zeroizing::new(password.to_owned())));
                    let incorrect = store.sign_sync(
                        signing_key(&handle),
                        &digest,
                        Some(Zeroizing::new("wrong password".to_owned())),
                    );
                }
                then "Accept only the correct Password" {
                    assert!(correct.is_ok());
                    assert!(incorrect.is_err());
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "EsapiDeviceKeyService::change_key_protection_sync" {
            case "Change Password without changing the public key" {
                given "Prepare DeviceKeyAlgorithm and ZEROIZING" {
                    let _lock = TPM_LOCK.lock().unwrap_or_else(|e| e.into_inner());
                    let store = require_swtpm();
                    let DeviceKeyMaterial {
                        handle: unprotected,
                        public_key: original_public,
                        ..
                    } = store
                        .generate_key_sync(DeviceKeyAlgorithm::EcdsaP256, None)
                        .unwrap();
                }
                when "Configure Password protection to Device Key" {
                    let first_password = Zeroizing::new("first password".to_owned());
                    let DeviceKeyMaterial {
                        handle: protected,
                        public_key: protected_public,
                        ..
                    } = store
                        .change_key_protection_sync(&unprotected, None, Some(first_password.clone()))
                        .unwrap();
                }
                then "You can still sign and sign after password protection" {
                    assert_eq!(
                        original_public.public_key_bytes(),
                        protected_public.public_key_bytes()
                    );
                    assert!(
                        store
                            .sign_sync(
                                signing_key(&protected),
                                &sha2_digest(b"protected"),
                                Some(first_password.clone())
                            )
                            .is_ok()
                    );
                }
                when "Change the Password for Protected Device Key" {
                    let second_password = Zeroizing::new("second password".to_owned());
                    let DeviceKeyMaterial {
                        handle: changed,
                        public_key: changed_public,
                        ..
                    } = store
                        .change_key_protection_sync(
                            &protected,
                            Some(first_password.clone()),
                            Some(second_password.clone()),
                        )
                        .unwrap();
                }
                then "Changed to new Password keeps public key and old Password rejected" {
                    assert_eq!(
                        original_public.public_key_bytes(),
                        changed_public.public_key_bytes()
                    );
                    assert!(
                        store
                            .sign_sync(signing_key(&changed), &sha2_digest(b"old"), Some(first_password))
                            .is_err()
                    );
                    assert!(
                        store
                            .sign_sync(
                                signing_key(&changed),
                                &sha2_digest(b"new"),
                                Some(second_password.clone())
                            )
                            .is_ok()
                    );
                }
                when "Deactivate PASSWORD Protection for Device Key" {
                    let DeviceKeyMaterial {
                        handle: unprotected_again,
                        public_key: final_public,
                        ..
                    } = store
                        .change_key_protection_sync(&changed, Some(second_password), None)
                        .unwrap();
                }
                then "You can sign with the same public key even if you unprotect Password" {
                    assert_eq!(
                        original_public.public_key_bytes(),
                        final_public.public_key_bytes()
                    );
                    assert!(
                        store
                            .sign_sync(signing_key(&unprotected_again), &sha2_digest(b"unprotected"), None)
                            .is_ok()
                    );
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "EsapiDeviceKeyService::sign" {
            case "A bad Key Handle signature returns an ERROR" {
                async: tokio;
                given "Prepare an unauthorized Device Key Handle" {
                    let store = EsapiDeviceKeyService::new(TctiNameConf::Swtpm(Default::default()), None);
                    let handle = DevicePrivateKeyHandle::new(vec![0u8; 10]);
                }
                when "Request a signature with a bad handle" {
                    let result = store.sign(&(), keyringd_core::service::DeviceKeySigningRequest {
                        handle: &handle,
                        profile_name: "test",
                        digest: &|_| [0; 32],
                    }).await;
                }
                then "Returns an ERROR without generating a signature" {
                    assert!(matches!(result, Err(EsapiError::InvalidKeyHandle(_))));
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "EsapiDeviceKeyService::generate_key_sync" {
            case "The two keys generated are different" {
                given "Prepare the available Software TPM" {
                    let _lock = TPM_LOCK.lock().unwrap_or_else(|e| e.into_inner());
                    let store = require_swtpm();
                }
                when "Generate two ECDSA Device Keys" {
                    let DeviceKeyMaterial {
                        handle: _h1,
                        public_key: pub1,
                        ..
                    } = store
                        .generate_key_sync(DeviceKeyAlgorithm::EcdsaP256, None)
                        .unwrap();
                    let DeviceKeyMaterial {
                        handle: _h2,
                        public_key: pub2,
                        ..
                    } = store
                        .generate_key_sync(DeviceKeyAlgorithm::EcdsaP256, None)
                        .unwrap();
                }
                then "Return a different public key" {
                    assert_ne!(pub1.public_key_bytes(), pub2.public_key_bytes());
                }
            }
        }
    }
}
