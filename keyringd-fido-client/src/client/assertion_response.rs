use super::{FidoClientError, ResponseValidationError};
use keyringd_fido::webauthn::LargeBlobOutput;
use keyringd_fido::{
    AuthenticatorData,
    ctap::{AuthenticatorGetAssertionRequest, AuthenticatorGetAssertionResponse},
    webauthn::{
        AuthenticationExtensionsAuthenticatorOutputs, PublicKeyCredentialDescriptor,
        PublicKeyCredentialType, PublicKeyCredentialUserEntity, UserVerificationRequirement,
    },
};
use sha2::{Digest, Sha256};

/// An assertion whose signing credential has been resolved from the response or request.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AssertionResponse {
    large_blob: Option<LargeBlobOutput>,
    credential: PublicKeyCredentialDescriptor,
    response: AuthenticatorGetAssertionResponse,
}

impl AssertionResponse {
    pub fn large_blob(&self) -> Option<&LargeBlobOutput> {
        self.large_blob.as_ref()
    }
    pub(crate) fn set_large_blob(&mut self, value: LargeBlobOutput) {
        self.large_blob = Some(value);
    }
    pub fn from_ctap(
        response: AuthenticatorGetAssertionResponse,
        allow_list: Option<&[PublicKeyCredentialDescriptor]>,
    ) -> Result<Self, FidoClientError> {
        let credential = match (response.credential(), allow_list) {
            (Some(credential), _) => credential.clone(),
            (None, Some([credential])) => credential.clone(),
            (None, _) => {
                return Err(FidoClientError::protocol(
                    ResponseValidationError::MissingCredential,
                ));
            }
        };
        Ok(Self {
            large_blob: None,
            credential,
            response,
        })
    }
    pub(crate) fn validate_for(
        &self,
        request: &AuthenticatorGetAssertionRequest,
        user_verification: &UserVerificationRequirement,
    ) -> Result<(), FidoClientError> {
        let data = self.authenticator_data();
        if data.rp_id_hash() != &<[u8; 32]>::from(Sha256::digest(request.rp_id().as_bytes())) {
            return Err(FidoClientError::protocol(
                ResponseValidationError::RpIdMismatch,
            ));
        }
        if self.credential().credential_type() != &PublicKeyCredentialType::PublicKey
            || self.credential().id().is_empty()
            || request.allow_list().is_some_and(|list| {
                !list
                    .iter()
                    .any(|credential| credential.id() == self.credential().id())
            })
        {
            return Err(FidoClientError::protocol(
                ResponseValidationError::UnexpectedCredential,
            ));
        }
        if request
            .options()
            .and_then(|options| options.get("up"))
            .copied()
            .unwrap_or(true)
            && !data.user_present()
        {
            return Err(FidoClientError::protocol(
                ResponseValidationError::UserPresenceMissing,
            ));
        }
        if user_verification == &UserVerificationRequirement::Required && !data.user_verified() {
            return Err(FidoClientError::protocol(
                ResponseValidationError::UserVerificationMissing,
            ));
        }
        if data.attested_credential_data().is_some() || self.signature().is_empty() {
            return Err(FidoClientError::protocol(
                ResponseValidationError::InvalidAssertion,
            ));
        }
        if request.allow_list().is_none() && self.user().is_none() {
            return Err(FidoClientError::protocol(
                ResponseValidationError::MissingDiscoverableUserEntity,
            ));
        }
        Ok(())
    }

    pub fn credential(&self) -> &PublicKeyCredentialDescriptor {
        &self.credential
    }
    pub fn authenticator_data(&self) -> &AuthenticatorData {
        self.response.authenticator_data()
    }
    pub fn signature(&self) -> &[u8] {
        self.response.signature()
    }
    pub fn user(&self) -> Option<&PublicKeyCredentialUserEntity> {
        self.response.user()
    }
    pub fn number_of_credentials(&self) -> Option<u64> {
        self.response.number_of_credentials()
    }
    pub fn user_selected(&self) -> Option<bool> {
        self.response.user_selected()
    }
    pub fn large_blob_key(&self) -> Option<&[u8]> {
        self.response.large_blob_key()
    }
    pub fn unsigned_extension_outputs(
        &self,
    ) -> Option<&AuthenticationExtensionsAuthenticatorOutputs> {
        self.response.unsigned_extension_outputs()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use keyringd_fido::webauthn::PublicKeyCredentialType;

    fn credential(id: u8) -> PublicKeyCredentialDescriptor {
        PublicKeyCredentialDescriptor::new(PublicKeyCredentialType::PublicKey, vec![id])
    }
    fn wire(
        credential: Option<PublicKeyCredentialDescriptor>,
    ) -> AuthenticatorGetAssertionResponse {
        AuthenticatorGetAssertionResponse::new(
            credential,
            AuthenticatorData::new("example.com", 1, 0).unwrap(),
            vec![1],
        )
    }
    #[test]
    fn resolves_only_an_unambiguous_request_and_preserves_an_explicit_credential() {
        let a = credential(1);
        let b = credential(2);
        let one = [a.clone()];
        let two = [a.clone(), b.clone()];
        let response = AssertionResponse::from_ctap(wire(None), Some(&one)).unwrap();
        assert_eq!(response.credential(), &a);
        assert_eq!(
            response.authenticator_data().raw(),
            wire(None).authenticator_data().raw()
        );
        for allow_list in [None, Some(&[][..]), Some(two.as_slice())] {
            let error = AssertionResponse::from_ctap(wire(None), allow_list).unwrap_err();
            assert_eq!(
                std::error::Error::source(&error)
                    .unwrap()
                    .downcast_ref::<ResponseValidationError>(),
                Some(&ResponseValidationError::MissingCredential)
            );
        }
        for allow_list in [None, Some(one.as_slice()), Some(two.as_slice())] {
            assert_eq!(
                AssertionResponse::from_ctap(wire(Some(b.clone())), allow_list)
                    .unwrap()
                    .credential(),
                &b
            );
        }
    }
}
