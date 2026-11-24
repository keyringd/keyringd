use super::{AssertionResponse, ClientData};
use keyringd_fido::webauthn::PublicKeyCredentialDescriptor;

/// A selected CTAP assertion and the exact client data used for its ceremony.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GetAssertionResult {
    response: AssertionResponse,
    client_data: ClientData,
}
impl GetAssertionResult {
    pub(crate) fn new(response: AssertionResponse, client_data: ClientData) -> Self {
        Self {
            response,
            client_data,
        }
    }
    /// Exact authenticator data followed by SHA-256 of the original client data.
    pub fn signed_data(&self) -> Vec<u8> {
        use sha2::{Digest, Sha256};
        let mut data = self.response.authenticator_data().raw().to_vec();
        data.extend_from_slice(&Sha256::digest(self.client_data.as_bytes()));
        data
    }
    pub fn response(&self) -> &AssertionResponse {
        &self.response
    }
    pub fn credential(&self) -> &PublicKeyCredentialDescriptor {
        self.response.credential()
    }
    pub fn client_data(&self) -> &ClientData {
        &self.client_data
    }
}
