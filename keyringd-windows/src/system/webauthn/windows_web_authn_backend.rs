use futures_util::future::BoxFuture;
use keyringd_fido::webauthn::LargeBlobOutput;
use keyringd_fido::webauthn::{LargeBlobInput, LargeBlobSupport};

use std::sync::Arc;
use tokio_util::sync::CancellationToken;

use windows::Win32::Foundation::HWND;
use windows::Win32::Networking::WindowsWebServices::{
    WEBAUTHN_API_VERSION_7, WEBAUTHN_ASSERTION_CURRENT_VERSION,
    WEBAUTHN_ATTESTATION_CONVEYANCE_PREFERENCE_DIRECT,
    WEBAUTHN_ATTESTATION_CONVEYANCE_PREFERENCE_INDIRECT,
    WEBAUTHN_ATTESTATION_CONVEYANCE_PREFERENCE_NONE,
    WEBAUTHN_AUTHENTICATOR_ATTACHMENT_CROSS_PLATFORM, WEBAUTHN_AUTHENTICATOR_GET_ASSERTION_OPTIONS,
    WEBAUTHN_AUTHENTICATOR_GET_ASSERTION_OPTIONS_VERSION_5,
    WEBAUTHN_AUTHENTICATOR_MAKE_CREDENTIAL_OPTIONS,
    WEBAUTHN_AUTHENTICATOR_MAKE_CREDENTIAL_OPTIONS_VERSION_4, WEBAUTHN_CLIENT_DATA,
    WEBAUTHN_CLIENT_DATA_CURRENT_VERSION, WEBAUTHN_COSE_ALGORITHM_ECDSA_P256_WITH_SHA256,
    WEBAUTHN_COSE_CREDENTIAL_PARAMETER, WEBAUTHN_COSE_CREDENTIAL_PARAMETER_CURRENT_VERSION,
    WEBAUTHN_COSE_CREDENTIAL_PARAMETERS, WEBAUTHN_CRED_LARGE_BLOB_OPERATION_GET,
    WEBAUTHN_CRED_LARGE_BLOB_OPERATION_SET, WEBAUTHN_CRED_LARGE_BLOB_STATUS_SUCCESS,
    WEBAUTHN_CRED_PROTECT_EXTENSION_IN, WEBAUTHN_CREDENTIAL,
    WEBAUTHN_CREDENTIAL_ATTESTATION_CURRENT_VERSION, WEBAUTHN_CREDENTIAL_CURRENT_VERSION,
    WEBAUTHN_CREDENTIAL_EX, WEBAUTHN_CREDENTIAL_EX_CURRENT_VERSION, WEBAUTHN_CREDENTIAL_LIST,
    WEBAUTHN_CREDENTIAL_TYPE_PUBLIC_KEY, WEBAUTHN_CREDENTIALS, WEBAUTHN_CTAP_TRANSPORT_BLE,
    WEBAUTHN_CTAP_TRANSPORT_HYBRID, WEBAUTHN_CTAP_TRANSPORT_INTERNAL, WEBAUTHN_CTAP_TRANSPORT_NFC,
    WEBAUTHN_CTAP_TRANSPORT_USB, WEBAUTHN_EXTENSION, WEBAUTHN_EXTENSIONS,
    WEBAUTHN_EXTENSIONS_IDENTIFIER_CRED_PROTECT, WEBAUTHN_HASH_ALGORITHM_SHA_256,
    WEBAUTHN_LARGE_BLOB_SUPPORT_PREFERRED, WEBAUTHN_LARGE_BLOB_SUPPORT_REQUIRED,
    WEBAUTHN_RP_ENTITY_INFORMATION, WEBAUTHN_RP_ENTITY_INFORMATION_CURRENT_VERSION,
    WEBAUTHN_USER_ENTITY_INFORMATION, WEBAUTHN_USER_ENTITY_INFORMATION_CURRENT_VERSION,
    WEBAUTHN_USER_VERIFICATION_REQUIREMENT_DISCOURAGED,
    WEBAUTHN_USER_VERIFICATION_REQUIREMENT_PREFERRED,
    WEBAUTHN_USER_VERIFICATION_REQUIREMENT_REQUIRED, WebAuthNAuthenticatorGetAssertion,
    WebAuthNAuthenticatorMakeCredential, WebAuthNGetApiVersionNumber,
};
use windows::core::{GUID, PCWSTR};

use super::AssertionWrapper;
use super::CredentialAttestation;
use super::GetAssertionOptionsV8;
use super::MakeCredentialOptionsV8;
use super::WebAuthnError;
use super::web_authn_operation::WebAuthnOperation;
pub use super::web_authn_window::WebAuthnWindow;
use keyringd_fido::ctap::AuthenticatorGetAssertionResponse;
use keyringd_fido::webauthn::{
    AttestationObject, AttestationStatement, AuthenticationExtensionsClientOutputs,
    AuthenticatorAttachment, AuthenticatorAttestationResponse, AuthenticatorData,
    PublicKeyCredentialCreationOptions, PublicKeyCredentialRequestOptions, PublicKeyCredentialType,
};
use keyringd_fido::webauthn::{PublicKeyCredentialDescriptor, PublicKeyCredentialUserEntity};
use keyringd_fido::{
    AttestationConveyancePreference, AuthenticatorTransport, ResidentKeyRequirement,
    UserVerificationRequirement,
};
use keyringd_fido_client::backend::OsUsbBackend;
use keyringd_fido_client::{
    ClientData, FidoClientError, GetAssertionsResult, MakeCredentialResult, OperationId,
};

const GET_ASSERTION_OPTIONS_VERSION_8: u32 = 8;
const MAKE_CREDENTIAL_OPTIONS_VERSION_8: u32 = 8;

// API v7 must support both SDK response types we borrow.
const _: () = {
    assert!(WEBAUTHN_CREDENTIAL_ATTESTATION_CURRENT_VERSION == 6);
    assert!(WEBAUTHN_ASSERTION_CURRENT_VERSION == 5);
};

pub struct WindowsWebAuthnBackend<W> {
    window: Arc<W>,
}

impl<W> WindowsWebAuthnBackend<W>
where
    W: WebAuthnWindow,
{
    pub fn new(window: Arc<W>) -> Self {
        Self { window }
    }

    fn window(&self) -> HWND {
        self.window.handle()
    }
}

impl<W> WindowsWebAuthnBackend<W>
where
    W: WebAuthnWindow,
{
    fn make_credential_impl(
        &self,
        api_version: u32,
        mut cancellation_id: GUID,
        cancellation: &CancellationToken,
        request: PublicKeyCredentialCreationOptions,
        client_data: ClientData,
    ) -> Result<MakeCredentialResult, FidoClientError> {
        let relying_party_id = request.rp().id().ok_or_else(|| {
            FidoClientError::UnsupportedRequirement("Windows WebAuthn requires an RP ID".into())
        })?;
        let rp_id = wide_string(relying_party_id);
        let rp_name = wide_string(request.rp().name());
        let user_name = wide_string(request.user().name().expect(
            "Passkey creation requests should provide the user name required by Windows WebAuthn",
        ));
        let user_display_name = wide_string(
            request
                .user()
                .display_name()
                .expect("Passkey creation requests should provide the display name required by Windows WebAuthn"),
        );
        let mut client_data_bytes = client_data.as_bytes().to_vec();

        let rp_info = WEBAUTHN_RP_ENTITY_INFORMATION {
            dwVersion: WEBAUTHN_RP_ENTITY_INFORMATION_CURRENT_VERSION,
            pwszId: PCWSTR(rp_id.as_ptr()),
            pwszName: PCWSTR(rp_name.as_ptr()),
            pwszIcon: PCWSTR::null(),
        };
        let user_info = WEBAUTHN_USER_ENTITY_INFORMATION {
            dwVersion: WEBAUTHN_USER_ENTITY_INFORMATION_CURRENT_VERSION,
            cbId: to_u32(request.user().id().len()).map_err(backend_error)?,
            pbId: request.user().id().as_ptr() as *mut u8,
            pwszName: PCWSTR(user_name.as_ptr()),
            pwszIcon: PCWSTR::null(),
            pwszDisplayName: PCWSTR(user_display_name.as_ptr()),
        };
        let mut cose_parameter = WEBAUTHN_COSE_CREDENTIAL_PARAMETER {
            dwVersion: WEBAUTHN_COSE_CREDENTIAL_PARAMETER_CURRENT_VERSION,
            pwszCredentialType: WEBAUTHN_CREDENTIAL_TYPE_PUBLIC_KEY,
            lAlg: WEBAUTHN_COSE_ALGORITHM_ECDSA_P256_WITH_SHA256,
        };
        let cose_parameters = WEBAUTHN_COSE_CREDENTIAL_PARAMETERS {
            cCredentialParameters: 1,
            pCredentialParameters: &mut cose_parameter,
        };
        let native_client_data = WEBAUTHN_CLIENT_DATA {
            dwVersion: WEBAUTHN_CLIENT_DATA_CURRENT_VERSION,
            cbClientDataJSON: to_u32(client_data_bytes.len()).map_err(backend_error)?,
            pbClientDataJSON: client_data_bytes.as_mut_ptr(),
            pwszHashAlgId: WEBAUTHN_HASH_ALGORITHM_SHA_256,
        };
        let mut excluded_ids = request
            .exclude_credentials()
            .iter()
            .map(|credential| credential.id().to_vec())
            .collect::<Vec<_>>();
        let mut excluded_credentials = excluded_ids
            .iter_mut()
            .map(|id| {
                Ok(WEBAUTHN_CREDENTIAL {
                    dwVersion: WEBAUTHN_CREDENTIAL_CURRENT_VERSION,
                    cbId: to_u32(id.len())?,
                    pbId: id.as_mut_ptr(),
                    pwszCredentialType: WEBAUTHN_CREDENTIAL_TYPE_PUBLIC_KEY,
                })
            })
            .collect::<Result<Vec<_>, WebAuthnError>>()
            .map_err(backend_error)?;
        let credential_protection_policy = request
            .extensions()
            .as_ref()
            .and_then(|extensions| extensions.credential_protection_policy());
        let mut credential_protection =
            credential_protection_policy.map(|policy| WEBAUTHN_CRED_PROTECT_EXTENSION_IN {
                dwCredProtect: encode_credential_protection(policy),
                bRequireCredProtect: request
                    .extensions()
                    .as_ref()
                    .and_then(|extensions| extensions.enforce_credential_protection_policy())
                    .unwrap_or(false)
                    .into(),
            });
        let mut extensions = credential_protection
            .as_mut()
            .map(|extension| WEBAUTHN_EXTENSION {
                pwszExtensionIdentifier: WEBAUTHN_EXTENSIONS_IDENTIFIER_CRED_PROTECT,
                cbExtension: std::mem::size_of::<WEBAUTHN_CRED_PROTECT_EXTENSION_IN>() as u32,
                pvExtension: std::ptr::from_mut(extension).cast(),
            })
            .into_iter()
            .collect::<Vec<_>>();
        let large_blob = request.extensions().and_then(|e| e.large_blob());
        if let Some(input) = large_blob {
            input
                .validate_registration()
                .map_err(|error| FidoClientError::Protocol(error.into()))?;
        }
        let use_credential_hint = api_version >= 8;
        let options = WEBAUTHN_AUTHENTICATOR_MAKE_CREDENTIAL_OPTIONS {
            dwVersion: if use_credential_hint {
                MAKE_CREDENTIAL_OPTIONS_VERSION_8
            } else {
                WEBAUTHN_AUTHENTICATOR_MAKE_CREDENTIAL_OPTIONS_VERSION_4
            },
            dwTimeoutMilliseconds: request.timeout().unwrap_or(120_000).max(1),
            CredentialList: WEBAUTHN_CREDENTIALS {
                cCredentials: to_u32(excluded_credentials.len()).map_err(backend_error)?,
                pCredentials: excluded_credentials.as_mut_ptr(),
            },
            Extensions: WEBAUTHN_EXTENSIONS {
                cExtensions: to_u32(extensions.len()).map_err(backend_error)?,
                pExtensions: extensions.as_mut_ptr(),
            },
            dwAuthenticatorAttachment: WEBAUTHN_AUTHENTICATOR_ATTACHMENT_CROSS_PLATFORM,
            bRequireResidentKey: request
                .authenticator_selection()
                .as_ref()
                .is_some_and(|selection| {
                    selection.resident_key() == Some(&ResidentKeyRequirement::Required)
                        || selection.require_resident_key()
                })
                .into(),
            dwUserVerificationRequirement: encode_user_verification(
                request
                    .authenticator_selection()
                    .as_ref()
                    .map_or(UserVerificationRequirement::Preferred, |selection| {
                        selection.user_verification().clone()
                    }),
            ),
            dwLargeBlobSupport: match large_blob {
                Some(input) if input.support() == Some(&LargeBlobSupport::Required) => {
                    WEBAUTHN_LARGE_BLOB_SUPPORT_REQUIRED
                }
                Some(_) => WEBAUTHN_LARGE_BLOB_SUPPORT_PREFERRED,
                _ => 0,
            },
            dwAttestationConveyancePreference: encode_attestation(request.attestation().clone()),
            pCancellationId: &mut cancellation_id,
            ..Default::default()
        };
        let mut credential_hints = [windows::core::w!("security-key")];
        let options = MakeCredentialOptionsV8 {
            base: options,
            prf_global_eval: std::ptr::null_mut(),
            credential_hint_count: u32::from(use_credential_hint),
            credential_hints: if use_credential_hint {
                credential_hints.as_mut_ptr()
            } else {
                std::ptr::null_mut()
            },
            third_party_payment: false.into(),
        };

        let window = self.window();
        let _owner_session = self.window.activate();
        if cancellation.is_cancelled() {
            return Err(FidoClientError::Cancelled);
        }
        let result = unsafe {
            WebAuthNAuthenticatorMakeCredential(
                window,
                &rp_info,
                &user_info,
                &cose_parameters,
                &native_client_data,
                Some(&options.base),
            )
        };
        let pointer = result.map_err(WebAuthnError::from).map_err(backend_error)?;
        let attestation = CredentialAttestation::new(pointer).map_err(backend_error)?;
        // Older responses may be shorter than the SDK struct. Read only dwVersion first.
        let version = unsafe { (*attestation.pointer.as_ptr()).dwVersion };
        require_webauthn_version(
            "registration response",
            version,
            WEBAUTHN_CREDENTIAL_ATTESTATION_CURRENT_VERSION,
        )?;
        let value = unsafe { attestation.pointer.as_ref() };
        let credential_id = unsafe { copy_bytes(value.pbCredentialId, value.cbCredentialId) }
            .map_err(backend_error)?;
        let authenticator_data =
            unsafe { copy_bytes(value.pbAuthenticatorData, value.cbAuthenticatorData) }
                .map_err(backend_error)?;
        let authenticator_data = AuthenticatorData::from_bytes(&authenticator_data)
            .map_err(WebAuthnError::InvalidAuthenticatorData)
            .map_err(backend_error)?;
        let format = unsafe { value.pwszFormatType.to_string() }
            .map_err(WebAuthnError::InvalidAttestationFormat)
            .map_err(backend_error)?;
        let encoded_attestation = unsafe { copy_bytes(value.pbAttestation, value.cbAttestation) }
            .map_err(backend_error)?;
        let attestation_statement = AttestationStatement::from_cbor_bytes(
            &format,
            if encoded_attestation.is_empty() {
                &[0xa0]
            } else {
                &encoded_attestation
            },
        )
        .map_err(WebAuthnError::InvalidAttestation)
        .map_err(backend_error)?;

        let mut transports = parse_transports(value.dwUsedTransport);
        if !transports.contains(&AuthenticatorTransport::usb()) {
            return Err(backend_error(WebAuthnError::NonUsbAuthenticator));
        }
        transports.sort_by(|left, right| left.as_str().cmp(right.as_str()));
        let response = AuthenticatorAttestationResponse::from_attestation_object(
            client_data.as_bytes().to_vec(),
            AttestationObject::new(authenticator_data, attestation_statement),
            transports,
        )
        .map_err(FidoClientError::backend)?;
        Ok(MakeCredentialResult::new(
            credential_id,
            response,
            Some(AuthenticatorAttachment::CrossPlatform),
            if large_blob.is_some() {
                AuthenticationExtensionsClientOutputs::default().with_large_blob(
                    LargeBlobOutput::Supported(value.bLargeBlobSupported.as_bool()),
                )
            } else {
                AuthenticationExtensionsClientOutputs::default()
            },
        ))
    }

    fn get_assertions_impl(
        &self,
        api_version: u32,
        mut cancellation_id: GUID,
        cancellation: &CancellationToken,
        request: PublicKeyCredentialRequestOptions,
        client_data_value: ClientData,
    ) -> Result<GetAssertionsResult, FidoClientError> {
        let relying_party_id = request.relying_party_id().ok_or_else(|| {
            FidoClientError::UnsupportedRequirement("Windows WebAuthn requires an RP ID".into())
        })?;
        let rp_id = wide_string(relying_party_id);
        let client_data_json = client_data_value.as_bytes().to_vec();
        let mut client_data = client_data_json.clone();
        let native_client_data = WEBAUTHN_CLIENT_DATA {
            dwVersion: WEBAUTHN_CLIENT_DATA_CURRENT_VERSION,
            cbClientDataJSON: to_u32(client_data.len()).map_err(backend_error)?,
            pbClientDataJSON: client_data.as_mut_ptr(),
            pwszHashAlgId: WEBAUTHN_HASH_ALGORITHM_SHA_256,
        };
        let allow_credentials = request.allow_credentials();
        let use_extended_list = allow_credentials
            .iter()
            .any(|credential| credential.transports().is_some());
        let mut allowed_ids = allow_credentials
            .iter()
            .map(|credential| credential.id().to_vec())
            .collect::<Vec<_>>();
        let mut allowed_credentials = if use_extended_list {
            Vec::new()
        } else {
            allowed_ids
                .iter_mut()
                .map(|id| {
                    Ok(WEBAUTHN_CREDENTIAL {
                        dwVersion: WEBAUTHN_CREDENTIAL_CURRENT_VERSION,
                        cbId: to_u32(id.len())?,
                        pbId: id.as_mut_ptr(),
                        pwszCredentialType: WEBAUTHN_CREDENTIAL_TYPE_PUBLIC_KEY,
                    })
                })
                .collect::<Result<Vec<_>, WebAuthnError>>()
                .map_err(backend_error)?
        };
        let mut allowed_credentials_ex = if use_extended_list {
            allowed_ids
                .iter_mut()
                .zip(allow_credentials)
                .map(|(id, credential)| {
                    Ok(WEBAUTHN_CREDENTIAL_EX {
                        dwVersion: WEBAUTHN_CREDENTIAL_EX_CURRENT_VERSION,
                        cbId: to_u32(id.len())?,
                        pbId: id.as_mut_ptr(),
                        pwszCredentialType: WEBAUTHN_CREDENTIAL_TYPE_PUBLIC_KEY,
                        dwTransports: encode_transports(
                            credential.transports().unwrap_or_default(),
                        ),
                    })
                })
                .collect::<Result<Vec<_>, WebAuthnError>>()
                .map_err(backend_error)?
        } else {
            Vec::new()
        };
        let credential_list = WEBAUTHN_CREDENTIALS {
            cCredentials: to_u32(allowed_credentials.len()).map_err(backend_error)?,
            pCredentials: allowed_credentials.as_mut_ptr(),
        };
        let mut allowed_credential_ex_pointers = allowed_credentials_ex
            .iter_mut()
            .map(std::ptr::from_mut)
            .collect::<Vec<_>>();
        let mut extended_credential_list = use_extended_list.then_some(WEBAUTHN_CREDENTIAL_LIST {
            cCredentials: allowed_credential_ex_pointers.len() as u32,
            ppCredentials: allowed_credential_ex_pointers.as_mut_ptr(),
        });
        let large_blob = request.extensions().and_then(|e| e.large_blob());
        if let Some(input) = large_blob {
            input
                .validate_authentication()
                .map_err(|error| FidoClientError::Protocol(error.into()))?;
        }
        let use_credential_hint = api_version >= 8;
        let options = WEBAUTHN_AUTHENTICATOR_GET_ASSERTION_OPTIONS {
            dwVersion: if use_credential_hint {
                GET_ASSERTION_OPTIONS_VERSION_8
            } else {
                WEBAUTHN_AUTHENTICATOR_GET_ASSERTION_OPTIONS_VERSION_5
            },
            dwTimeoutMilliseconds: request.timeout().unwrap_or(120_000).max(1),
            dwCredLargeBlobOperation: match large_blob {
                Some(input) if input.read() == Some(true) => WEBAUTHN_CRED_LARGE_BLOB_OPERATION_GET,
                Some(input) if input.write().is_some() => WEBAUTHN_CRED_LARGE_BLOB_OPERATION_SET,
                _ => 0,
            },
            cbCredLargeBlob: match large_blob.and_then(LargeBlobInput::write) {
                Some(data) => to_u32(data.len()).map_err(backend_error)?,
                _ => 0,
            },
            pbCredLargeBlob: match large_blob.and_then(LargeBlobInput::write) {
                Some(data) => data.as_ptr() as *mut u8,
                _ => std::ptr::null_mut(),
            },
            CredentialList: credential_list,
            pAllowCredentialList: extended_credential_list
                .as_mut()
                .map_or(std::ptr::null_mut(), |list| list),
            dwAuthenticatorAttachment: WEBAUTHN_AUTHENTICATOR_ATTACHMENT_CROSS_PLATFORM,
            dwUserVerificationRequirement: encode_user_verification(
                request.user_verification().clone(),
            ),
            pCancellationId: &mut cancellation_id,
            ..Default::default()
        };
        let mut credential_hints = [windows::core::w!("security-key")];
        let options = GetAssertionOptionsV8 {
            base: options,
            c_credential_hints: if use_credential_hint { 1 } else { 0 },
            credential_hints: if use_credential_hint {
                credential_hints.as_mut_ptr()
            } else {
                std::ptr::null_mut()
            },
        };

        let window = self.window();
        let _owner_session = self.window.activate();
        if cancellation.is_cancelled() {
            return Err(FidoClientError::Cancelled);
        }
        let result = unsafe {
            WebAuthNAuthenticatorGetAssertion(
                window,
                PCWSTR(rp_id.as_ptr()),
                &native_client_data,
                Some(&options.base),
            )
        };
        let pointer = result.map_err(WebAuthnError::from).map_err(backend_error)?;
        let assertion = AssertionWrapper::new(pointer).map_err(backend_error)?;
        let version = unsafe { (*assertion.pointer.as_ptr()).dwVersion };
        require_webauthn_version(
            "signature response",
            version,
            WEBAUTHN_ASSERTION_CURRENT_VERSION,
        )?;
        let value = unsafe { assertion.pointer.as_ref() };
        let authenticator_data =
            unsafe { copy_bytes(value.pbAuthenticatorData, value.cbAuthenticatorData) }
                .map_err(backend_error)?;
        let credential_id = unsafe { copy_bytes(value.Credential.pbId, value.Credential.cbId) }
            .map_err(backend_error)?;
        let user_handle = unsafe { copy_bytes(value.pbUserId, value.cbUserId) }
            .ok()
            .filter(|value| !value.is_empty());
        let signature =
            unsafe { copy_bytes(value.pbSignature, value.cbSignature) }.map_err(backend_error)?;
        if signature.is_empty() {
            return Err(backend_error(WebAuthnError::InvalidSignature));
        }

        let parsed_auth_data = AuthenticatorData::from_bytes(&authenticator_data)
            .map_err(WebAuthnError::InvalidAuthenticatorData)
            .map_err(backend_error)?;
        let used_transport_flags = value.dwUsedTransport;
        let transports = parse_transports(used_transport_flags);
        if !transports.contains(&AuthenticatorTransport::usb()) {
            return Err(backend_error(WebAuthnError::NonUsbAuthenticator));
        }
        tracing::debug!(
            api_version,
            used_transport_flags,
            ?transports,
            "WebAuthn assertion completed"
        );
        let credential =
            PublicKeyCredentialDescriptor::new(PublicKeyCredentialType::PublicKey, credential_id);
        let mut response =
            AuthenticatorGetAssertionResponse::new(Some(credential), parsed_auth_data, signature);
        if let Some(id) = user_handle {
            response = response.with_user(PublicKeyCredentialUserEntity::from_id(id));
        }
        let selected_id = response
            .credential()
            .expect("resolved Windows credential")
            .id()
            .to_vec();
        let mut result = GetAssertionsResult::from_ctap(
            vec![response],
            client_data_value,
            AuthenticatorTransport::usb(),
            None,
        )?;
        let success = value.dwCredLargeBlobStatus == WEBAUTHN_CRED_LARGE_BLOB_STATUS_SUCCESS;
        let output = match large_blob {
            Some(input) if input.read() == Some(true) => Some(LargeBlobOutput::Read(if success {
                unsafe { copy_bytes(value.pbCredLargeBlob, value.cbCredLargeBlob) }.ok()
            } else {
                None
            })),
            Some(input) if input.write().is_some() => Some(LargeBlobOutput::Written(success)),
            _ => None,
        };
        if let Some(output) = output {
            result = result.with_large_blob(&selected_id, output);
        }
        Ok(result)
    }
}

impl<W> OsUsbBackend for WindowsWebAuthnBackend<W>
where
    W: WebAuthnWindow + 'static,
{
    fn make_credential(
        &self,
        operation_id: OperationId,
        options: PublicKeyCredentialCreationOptions,
        client_data: ClientData,
        cancellation: CancellationToken,
    ) -> BoxFuture<'_, Result<MakeCredentialResult, FidoClientError>> {
        let worker = Self {
            window: Arc::clone(&self.window),
        };
        Box::pin(async move {
            let api_version = usb_api_version()?;
            WebAuthnOperation::new(operation_id)?
                .run(cancellation, move |id, cancellation| {
                    worker.make_credential_impl(
                        api_version,
                        id,
                        &cancellation,
                        options,
                        client_data,
                    )
                })
                .await
        })
    }

    fn get_assertions(
        &self,
        operation_id: OperationId,
        options: PublicKeyCredentialRequestOptions,
        client_data: ClientData,
        cancellation: CancellationToken,
    ) -> BoxFuture<'_, Result<GetAssertionsResult, FidoClientError>> {
        let worker = Self {
            window: Arc::clone(&self.window),
        };
        Box::pin(async move {
            let api_version = usb_api_version()?;
            WebAuthnOperation::new(operation_id)?
                .run(cancellation, move |id, cancellation| {
                    worker.get_assertions_impl(api_version, id, &cancellation, options, client_data)
                })
                .await
        })
    }
}

fn usb_api_version() -> Result<u32, FidoClientError> {
    let version = unsafe { WebAuthNGetApiVersionNumber() };
    require_webauthn_version("API", version, WEBAUTHN_API_VERSION_7)?;
    Ok(version)
}

fn require_webauthn_version(name: &str, version: u32, minimum: u32) -> Result<(), FidoClientError> {
    if version < minimum {
        return Err(FidoClientError::UnsupportedRequirement(format!(
            "USB security keys require Windows WebAuthn {name} version {minimum} or later (found {version})"
        ).into()));
    }
    Ok(())
}

fn encode_user_verification(value: UserVerificationRequirement) -> u32 {
    match value {
        UserVerificationRequirement::Required => WEBAUTHN_USER_VERIFICATION_REQUIREMENT_REQUIRED,
        UserVerificationRequirement::Preferred => WEBAUTHN_USER_VERIFICATION_REQUIREMENT_PREFERRED,
        UserVerificationRequirement::Discouraged => {
            WEBAUTHN_USER_VERIFICATION_REQUIREMENT_DISCOURAGED
        }
        UserVerificationRequirement::Unknown(_) => WEBAUTHN_USER_VERIFICATION_REQUIREMENT_PREFERRED,
    }
}

fn encode_credential_protection(
    value: &keyringd_fido::webauthn::CredentialProtectionPolicy,
) -> u32 {
    use keyringd_fido::webauthn::CredentialProtectionPolicy;

    match value {
        CredentialProtectionPolicy::UserVerificationOptional => 1,
        CredentialProtectionPolicy::UserVerificationOptionalWithCredentialIdList => 2,
        CredentialProtectionPolicy::UserVerificationRequired => 3,
        CredentialProtectionPolicy::Unknown(_) => 0,
    }
}

fn encode_attestation(value: AttestationConveyancePreference) -> u32 {
    match value {
        AttestationConveyancePreference::None => WEBAUTHN_ATTESTATION_CONVEYANCE_PREFERENCE_NONE,
        AttestationConveyancePreference::Indirect => {
            WEBAUTHN_ATTESTATION_CONVEYANCE_PREFERENCE_INDIRECT
        }
        AttestationConveyancePreference::Direct => {
            WEBAUTHN_ATTESTATION_CONVEYANCE_PREFERENCE_DIRECT
        }
        // The Windows headers define this WebAuthn enum value as 4 even though
        // the windows crate does not currently expose a named constant for it.
        AttestationConveyancePreference::Enterprise => 4,
        AttestationConveyancePreference::Unknown(_) => {
            WEBAUTHN_ATTESTATION_CONVEYANCE_PREFERENCE_NONE
        }
    }
}

unsafe fn copy_bytes(pointer: *const u8, length: u32) -> Result<Vec<u8>, WebAuthnError> {
    if length == 0 {
        return Ok(Vec::new());
    }
    if pointer.is_null() {
        return Err(WebAuthnError::NullResult);
    }
    Ok(unsafe { std::slice::from_raw_parts(pointer, length as usize) }.to_vec())
}

fn to_u32(value: usize) -> Result<u32, WebAuthnError> {
    value.try_into().map_err(|_| WebAuthnError::ValueTooLarge)
}

fn backend_error(error: WebAuthnError) -> FidoClientError {
    if error.is_user_cancellation() {
        FidoClientError::OsCancelled
    } else {
        FidoClientError::backend(error)
    }
}

fn wide_string(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

fn parse_transports(flags: u32) -> Vec<AuthenticatorTransport> {
    let mut result = Vec::new();
    if flags & WEBAUTHN_CTAP_TRANSPORT_USB != 0 {
        result.push(AuthenticatorTransport::usb());
    }
    if flags & WEBAUTHN_CTAP_TRANSPORT_NFC != 0 {
        result.push(AuthenticatorTransport::nfc());
    }
    if flags & WEBAUTHN_CTAP_TRANSPORT_BLE != 0 {
        result.push(AuthenticatorTransport::ble());
    }
    if flags & WEBAUTHN_CTAP_TRANSPORT_INTERNAL != 0 {
        result.push(AuthenticatorTransport::internal());
    }
    if flags & WEBAUTHN_CTAP_TRANSPORT_HYBRID != 0 {
        result.push(AuthenticatorTransport::hybrid());
    }
    result
}

fn encode_transports(transports: &[AuthenticatorTransport]) -> u32 {
    transports.iter().fold(0, |flags, transport| {
        flags
            | match transport.as_str() {
                "usb" => WEBAUTHN_CTAP_TRANSPORT_USB,
                "nfc" => WEBAUTHN_CTAP_TRANSPORT_NFC,
                "ble" => WEBAUTHN_CTAP_TRANSPORT_BLE,
                "internal" => WEBAUTHN_CTAP_TRANSPORT_INTERNAL,
                "hybrid" => WEBAUTHN_CTAP_TRANSPORT_HYBRID,
                _ => 0,
            }
    })
}
