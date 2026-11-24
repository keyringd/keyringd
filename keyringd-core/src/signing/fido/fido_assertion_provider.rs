use crate::application::ApplicationError;
use keyringd_fido_client::{ClientDataProvider, GetAssertionResult};
use std::sync::Arc;

/// Supplies FIDO assertions to SSH signers, independently of the acquisition backend.
pub(crate) trait FidoAssertionProvider: Send {
    fn get_assertion(
        &mut self,
        options: keyringd_fido::webauthn::PublicKeyCredentialRequestOptions,
        client_data_provider: Arc<dyn ClientDataProvider>,
    ) -> impl Future<Output = Result<GetAssertionResult, ApplicationError>> + Send;
}
