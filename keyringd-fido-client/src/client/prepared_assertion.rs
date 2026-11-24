use super::{ClientData, ClientDataProvider, FidoClientError, GetAssertionsResult};
use crate::AuthenticatorRequirements;
use keyringd_fido::UserVerificationRequirement;
use keyringd_fido::{
    ctap::{AuthenticatorGetAssertionRequest, ClientDataHash},
    webauthn::{
        PublicKeyCredentialDescriptor, PublicKeyCredentialRequestOptions, PublicKeyCredentialType,
    },
};

/// One prepared assertion ceremony. Client data is collected once and retained
/// alongside the corresponding CTAP request for all retries and response checks.
pub(crate) struct PreparedAssertion {
    pub(crate) options: PublicKeyCredentialRequestOptions,
    pub(crate) client_data: ClientData,
    requirements: AuthenticatorRequirements,
    ctap_request: AuthenticatorGetAssertionRequest,
}

impl PreparedAssertion {
    pub fn new(
        options: PublicKeyCredentialRequestOptions,
        provider: &dyn ClientDataProvider,
    ) -> Result<Self, FidoClientError> {
        if let Some(input) = options.extensions().and_then(|e| e.large_blob()) {
            input
                .validate_authentication()
                .map_err(FidoClientError::protocol)?;
            if input.write().is_some() && options.allow_credentials().len() != 1 {
                return Err(FidoClientError::UnsupportedRequirement(
                    "largeBlob write requires exactly one credential".into(),
                ));
            }
        }
        let client_data = provider
            .provide(options.challenge())
            .map_err(FidoClientError::protocol)?;
        let rp_id = options.relying_party_id().ok_or_else(|| {
            FidoClientError::UnsupportedRequirement(
                "a standalone FIDO client requires an explicit relying party ID".into(),
            )
        })?;
        let mut ctap_request = AuthenticatorGetAssertionRequest::new(
            rp_id,
            ClientDataHash::hash(client_data.as_bytes()),
        );
        let allow_credentials = Self::resolve_allow_credentials(&options)?;
        if let Some(credentials) = &allow_credentials {
            ctap_request = ctap_request.with_allow_list(credentials.clone());
        }
        Ok(Self {
            requirements: Self::resolve_requirements(&options),
            options: options.with_allow_credentials(allow_credentials.unwrap_or_default()),
            client_data,
            ctap_request,
        })
    }

    /// Defaults to user presence required, as in WebAuthn. Explicitly disabling
    /// presence is useful for non-WebAuthn clients such as SSH security-key signing.
    pub fn with_user_presence(mut self, required: bool) -> Self {
        self.ctap_request = self.ctap_request.with_option("up", required);
        self
    }
    pub fn ctap_request(&self) -> &AuthenticatorGetAssertionRequest {
        &self.ctap_request
    }
    pub(crate) fn user_verification(&self) -> &UserVerificationRequirement {
        &self.requirements.user_verification
    }

    pub(crate) fn authenticator_requirements(&self) -> &AuthenticatorRequirements {
        &self.requirements
    }
    fn resolve_user_verification(
        options: &PublicKeyCredentialRequestOptions,
    ) -> UserVerificationRequirement {
        match options.user_verification() {
            UserVerificationRequirement::Unknown(_) => UserVerificationRequirement::Preferred,
            value => value.clone(),
        }
    }
    fn resolve_requirements(
        options: &PublicKeyCredentialRequestOptions,
    ) -> AuthenticatorRequirements {
        AuthenticatorRequirements {
            discoverable_credential: true,
            user_verification: Self::resolve_user_verification(options),
            algorithms: vec![keyringd_fido::webauthn::CoseAlgorithmIdentifier::ES256],
            required_extensions: Vec::new(),
        }
    }
    /// Validates results obtained through an OS, transport, or application backend.
    pub fn validate_result(
        &self,
        result: GetAssertionsResult,
    ) -> Result<GetAssertionsResult, FidoClientError> {
        result.validate_for(
            &self.ctap_request,
            self.user_verification(),
            &self.client_data,
        )
    }

    pub(crate) fn resolve_allow_credentials(
        options: &PublicKeyCredentialRequestOptions,
    ) -> Result<Option<Vec<PublicKeyCredentialDescriptor>>, FidoClientError> {
        if options.allow_credentials().is_empty() {
            return Ok(None);
        }
        let supported = options
            .allow_credentials()
            .iter()
            .filter(|credential| {
                credential.credential_type() == &PublicKeyCredentialType::PublicKey
            })
            .cloned()
            .collect::<Vec<_>>();
        if supported.is_empty() {
            return Err(FidoClientError::UnsupportedRequirement(
                "allowCredentials contains no supported public-key credential".into(),
            ));
        }
        Ok(Some(supported))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AssertionResponse;
    use keyringd_fido::UserVerificationRequirement;
    use keyringd_fido::ctap::AuthenticatorGetAssertionResponse;
    use keyringd_fido::webauthn::{AuthenticatorData, AuthenticatorTransport};

    keyringd_testkit::scenario! {
        group "PreparedAssertion" {
            case "Omits redundant user presence while preparing CTAP from collected client data" {
                verify "preparation preserves CTAP interoperability and hashes client data rather than the challenge" {
                    let options = PublicKeyCredentialRequestOptions::new(vec![1]).with_relying_party_id("example.com");
                    let request = PreparedAssertion::new(options, &|_: &[u8]| Ok(ClientData::from_bytes(vec![2]))).unwrap();
                    assert!(request.ctap_request().options().is_none_or(|options| !options.contains_key("up")));
                    assert_eq!(request.ctap_request().client_data_hash(), &ClientDataHash::hash(&[2]));
                }
            }
            case "Rejects mismatched RP credential missing UP UV and stale client data" {
                verify "the shared client pipeline binds responses to the prepared ceremony" {
                    let credential = PublicKeyCredentialDescriptor::new(PublicKeyCredentialType::PublicKey, vec![1]);
                    let options = PublicKeyCredentialRequestOptions::new(vec![1]).with_relying_party_id("example.com").with_allow_credentials(vec![credential.clone()]).with_user_verification(UserVerificationRequirement::Required);
                    let request = PreparedAssertion::new(options, &|_: &[u8]| Ok(ClientData::from_bytes(vec![2]))).unwrap();
                    let response = AuthenticatorGetAssertionResponse::new(Some(credential.clone()), AuthenticatorData::new("example.com", 5, 0).unwrap(), vec![1;64]);
                    assert!(AssertionResponse::from_ctap(response.clone(), None).unwrap().validate_for(request.ctap_request(), request.options.user_verification()).is_ok());
                    for invalid in [
                        AuthenticatorGetAssertionResponse::new(Some(credential.clone()), AuthenticatorData::new("other.example", 5, 0).unwrap(), vec![1;64]),
                        AuthenticatorGetAssertionResponse::new(Some(PublicKeyCredentialDescriptor::new(PublicKeyCredentialType::PublicKey, vec![2])), AuthenticatorData::new("example.com", 5, 0).unwrap(), vec![1;64]),
                        AuthenticatorGetAssertionResponse::new(Some(credential.clone()), AuthenticatorData::new("example.com", 4, 0).unwrap(), vec![1;64]),
                        AuthenticatorGetAssertionResponse::new(Some(credential.clone()), AuthenticatorData::new("example.com", 1, 0).unwrap(), vec![1;64]),
                        AuthenticatorGetAssertionResponse::new(Some(credential), AuthenticatorData::new("example.com", 5, 0).unwrap(), vec![]),
                    ] { assert!(matches!(AssertionResponse::from_ctap(invalid, None).unwrap().validate_for(request.ctap_request(), request.options.user_verification()), Err(FidoClientError::Protocol(_)))); }
                    let stale = GetAssertionsResult::from_ctap(vec![response.clone()], ClientData::from_bytes(vec![3]), AuthenticatorTransport::usb(), None).unwrap();
                    assert!(matches!(request.validate_result(stale), Err(FidoClientError::Protocol(_))));
                    let wrong_count = GetAssertionsResult::from_ctap(vec![response.with_number_of_credentials(2)], request.client_data.clone(), AuthenticatorTransport::usb(), None).unwrap();
                    assert!(matches!(request.validate_result(wrong_count), Err(FidoClientError::Protocol(_))));
                }
            }
        }
    }
}
