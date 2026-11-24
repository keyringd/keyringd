use keyringd_fido::webauthn::LargeBlobOutput;
use std::collections::BTreeSet;

use keyringd_fido::webauthn::AuthenticatorTransport;
use keyringd_fido::{
    UserVerificationRequirement,
    ctap::{AuthenticatorGetAssertionRequest, AuthenticatorGetAssertionResponse},
};

use super::{AssertionResponse, ClientData, FidoClientError, ResponseValidationError};

/// Preserve the set of CTAP responses obtained by authentication and the ClientData/Communication path used throughout the operation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GetAssertionsResult {
    usb_device_id: Option<crate::usb::UsbHidDeviceId>,
    responses: Vec<AssertionResponse>,
    client_data: ClientData,
    transport: AuthenticatorTransport,
}

impl GetAssertionsResult {
    pub fn with_large_blob(mut self, credential_id: &[u8], output: LargeBlobOutput) -> Self {
        if let Some(response) = self
            .responses
            .iter_mut()
            .find(|r| r.credential().id() == credential_id)
        {
            response.set_large_blob(output);
        }
        self
    }
    pub fn from_ctap(
        responses: Vec<AuthenticatorGetAssertionResponse>,
        client_data: ClientData,
        transport: AuthenticatorTransport,
        allow_list: Option<&[keyringd_fido::webauthn::PublicKeyCredentialDescriptor]>,
    ) -> Result<Self, FidoClientError> {
        let responses = responses
            .into_iter()
            .map(|response| AssertionResponse::from_ctap(response, allow_list))
            .collect::<Result<_, _>>()?;
        Ok(Self {
            usb_device_id: None,
            responses,
            client_data,
            transport,
        })
    }

    /// A local identification value for specifying the same connected USB key for subsequent operations.
    pub fn usb_device_id(&self) -> Option<&crate::usb::UsbHidDeviceId> {
        self.usb_device_id.as_ref()
    }

    pub(crate) fn with_usb_device_id(mut self, id: Option<crate::usb::UsbHidDeviceId>) -> Self {
        self.usb_device_id = id;
        self
    }

    /// Selects a known credential, or the first response when no ID was requested.
    pub fn select(
        &self,
        credential_id: Option<&[u8]>,
    ) -> Result<super::GetAssertionResult, FidoClientError> {
        let response = self
            .responses
            .iter()
            .find(|response| credential_id.is_none_or(|id| response.credential().id() == id))
            .ok_or(FidoClientError::Ctap(
                keyringd_fido::CtapStatusCode::NoCredentials,
            ))?;
        Ok(super::GetAssertionResult::new(
            response.clone(),
            self.client_data.clone(),
        ))
    }

    pub fn responses(&self) -> &[AssertionResponse] {
        &self.responses
    }
    pub(super) fn responses_mut(&mut self) -> &mut [AssertionResponse] {
        &mut self.responses
    }
    pub(super) fn validate_for(
        self,
        request: &AuthenticatorGetAssertionRequest,
        user_verification: &UserVerificationRequirement,
        client_data: &ClientData,
    ) -> Result<Self, FidoClientError> {
        if self.client_data() != client_data {
            return Err(FidoClientError::protocol(
                ResponseValidationError::ClientDataMismatch,
            ));
        }
        let result = self.validate()?;
        for response in result.responses() {
            response.validate_for(request, user_verification)?;
        }
        if result.responses()[0]
            .number_of_credentials()
            .is_some_and(|count| count != result.responses().len() as u64)
        {
            return Err(FidoClientError::protocol(
                ResponseValidationError::CredentialCountMismatch,
            ));
        }
        Ok(result)
    }

    pub fn client_data(&self) -> &ClientData {
        &self.client_data
    }
    pub fn transport(&self) -> &AuthenticatorTransport {
        &self.transport
    }

    pub(crate) fn validate(self) -> Result<Self, FidoClientError> {
        if self.responses.is_empty() {
            return Err(FidoClientError::protocol(
                ResponseValidationError::NoAssertions,
            ));
        }
        let mut credential_ids = BTreeSet::new();
        for response in &self.responses {
            if !credential_ids.insert(response.credential().id()) {
                return Err(FidoClientError::protocol(
                    ResponseValidationError::DuplicateCredential,
                ));
            }
        }
        Ok(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    keyringd_testkit::scenario! {
        group "GetAssertionsResult::validate" {
            case "Rejects empty assertion results" {
                why "A successful FIDO operation must provide at least one assertion."
                given "Prepare results with an empty Assertion list" {
                    let result = GetAssertionsResult::from_ctap(
                        Vec::new(),
                        ClientData::from_bytes(Vec::new()),
                        AuthenticatorTransport::usb(), None).unwrap();
                }
                when "Verify the results of the operation" {
                    let error = result.validate().unwrap_err();
                }
                then "A protocol error caused by NOASSERTIONS" {
                    assert!(matches!(&error, crate::FidoClientError::Protocol(_)));
                    assert_eq!(
                        std::error::Error::source(&error)
                            .unwrap()
                            .downcast_ref::<ResponseValidationError>(),
                        Some(&ResponseValidationError::NoAssertions),
                    );
                }
            }
        }
    }
}
