use crate::{AuthenticatorCapabilityError, AuthenticatorRequest, AuthenticatorRequirements};
use keyringd_fido::ctap::{
    AuthenticatorGetAssertionRequest, AuthenticatorGetInfoResponse,
    AuthenticatorMakeCredentialRequest, CtapVersion,
};
use keyringd_fido::webauthn::{CoseAlgorithmIdentifier, PublicKeyCredentialType};

const DEFAULT_MAX_MESSAGE_SIZE: usize = 1024;

/// Capacity information of the authenticator acquired by GetInfo. Do not retain the PIN/UV method used in this operation.
/// Check whether you can respond to requests and the upper limit of communication/certification information.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthenticatorCapabilities {
    info: AuthenticatorGetInfoResponse,
}

impl AuthenticatorCapabilities {
    pub fn from_get_info(info: AuthenticatorGetInfoResponse) -> Self {
        Self { info }
    }

    pub(crate) fn info(&self) -> &AuthenticatorGetInfoResponse {
        &self.info
    }

    pub fn check_requirements(
        &self,
        requirements: &AuthenticatorRequirements,
        request: AuthenticatorRequest<'_>,
    ) -> Result<(), AuthenticatorCapabilityError> {
        let info = &self.info;
        if self.version().is_none() {
            return Err(AuthenticatorCapabilityError::UnsupportedVersion);
        }
        if requirements.discoverable_credential && !info.supports_discoverable_credentials() {
            return Err(AuthenticatorCapabilityError::DiscoverableCredentialsUnsupported);
        }
        if !requirements
            .algorithms
            .iter()
            .any(|algorithm| supports_algorithm(info, *algorithm))
        {
            return Err(AuthenticatorCapabilityError::AlgorithmUnsupported);
        }
        for extension in &requirements.required_extensions {
            if !info
                .extensions()
                .is_some_and(|values| values.contains(extension))
            {
                return Err(AuthenticatorCapabilityError::ExtensionUnsupported(
                    extension.clone(),
                ));
            }
        }
        let encoded = match request {
            AuthenticatorRequest::MakeCredential(request) => self
                .constrain_make_credential_request(request.clone())?
                .to_cbor_bytes(),
            AuthenticatorRequest::GetAssertion(request) => self
                .constrain_get_assertion_request(request.clone())?
                .to_cbor_bytes(),
        }
        .map_err(AuthenticatorCapabilityError::InvalidRequest)?;
        self.ensure_message_size(encoded.len() + 1)
    }

    pub(crate) fn version(&self) -> Option<CtapVersion> {
        self.info.highest_supported_version()
    }

    pub(crate) fn maximum_message_size(&self) -> usize {
        self.info
            .max_message_size()
            .map(|value| usize::try_from(value).unwrap_or(usize::MAX))
            .unwrap_or(DEFAULT_MAX_MESSAGE_SIZE)
    }
    pub(crate) fn maximum_credential_count_in_list(&self) -> Option<usize> {
        self.info
            .max_credential_count_in_list()
            .map(|value| usize::try_from(value).unwrap_or(usize::MAX))
    }
    pub(crate) fn maximum_credential_id_length(&self) -> Option<usize> {
        self.info
            .max_credential_id_length()
            .map(|value| usize::try_from(value).unwrap_or(usize::MAX))
    }
    pub(crate) fn supports_permission_tokens(&self) -> bool {
        self.info
            .options()
            .and_then(|options| options.get("pinUvAuthToken"))
            .copied()
            .unwrap_or(false)
    }
    pub(crate) fn preferred_platform_uv_attempts(&self) -> u64 {
        self.info.preferred_platform_uv_attempts().unwrap_or(1)
    }
    pub(crate) fn constrain_make_credential_request(
        &self,
        request: AuthenticatorMakeCredentialRequest,
    ) -> Result<AuthenticatorMakeCredentialRequest, AuthenticatorCapabilityError> {
        let Some(exclude_list) = request.exclude_list() else {
            return Ok(request);
        };
        let mut filtered = exclude_list.to_vec();
        if let Some(maximum) = self.maximum_credential_id_length() {
            filtered.retain(|credential| credential.id().len() <= maximum);
        }
        self.ensure_credential_list_size(filtered.len())?;
        Ok(request.with_exclude_list(filtered))
    }

    pub(crate) fn constrain_get_assertion_request(
        &self,
        request: AuthenticatorGetAssertionRequest,
    ) -> Result<AuthenticatorGetAssertionRequest, AuthenticatorCapabilityError> {
        let Some(allow_list) = request.allow_list() else {
            return Ok(request);
        };
        let mut filtered = allow_list.to_vec();
        if let Some(maximum) = self.maximum_credential_id_length() {
            filtered.retain(|credential| credential.id().len() <= maximum);
        }
        if filtered.is_empty() {
            return Err(AuthenticatorCapabilityError::NoMatchingCredential);
        }
        self.ensure_credential_list_size(filtered.len())?;
        Ok(request.with_allow_list(filtered))
    }

    pub(crate) fn ensure_credential_list_size(
        &self,
        actual: usize,
    ) -> Result<(), AuthenticatorCapabilityError> {
        if let Some(maximum) = self.maximum_credential_count_in_list()
            && actual > maximum
        {
            return Err(AuthenticatorCapabilityError::CredentialListTooLong { actual, maximum });
        }
        Ok(())
    }

    pub(crate) fn ensure_message_size(
        &self,
        required: usize,
    ) -> Result<(), AuthenticatorCapabilityError> {
        if required > self.maximum_message_size() {
            return Err(AuthenticatorCapabilityError::MessageTooLarge {
                required,
                maximum: self.maximum_message_size(),
            });
        }
        Ok(())
    }
}

fn supports_algorithm(
    info: &AuthenticatorGetInfoResponse,
    algorithm: CoseAlgorithmIdentifier,
) -> bool {
    match info.algorithms() {
        Some(algorithms) => algorithms.iter().any(|candidate| {
            candidate.credential_type() == &PublicKeyCredentialType::PublicKey
                && candidate.algorithm() == algorithm
        }),
        // ES256 is mandatory for FIDO2 authenticators predating the algorithms field.
        None => algorithm == CoseAlgorithmIdentifier::ES256,
    }
}
