use super::FidoClientError;
use keyringd_fido::ctap::{
    AuthenticatorGetAssertionRequest, AuthenticatorGetAssertionResponse,
    AuthenticatorMakeCredentialRequest, AuthenticatorMakeCredentialResponse,
};

/// An authenticator connected through the Internal route. It receives only CTAP data;
/// application-specific UI context is bound to the implementation, never serialized.
/// On cancellation, settle native operations and release their UI before returning.
pub trait InternalAuthenticator: Send + Sync {
    fn make_credential(
        &self,
        request: &AuthenticatorMakeCredentialRequest,
        cancellation: tokio_util::sync::CancellationToken,
    ) -> impl Future<Output = Result<AuthenticatorMakeCredentialResponse, FidoClientError>> + Send;

    fn get_assertion(
        &self,
        request: &AuthenticatorGetAssertionRequest,
        cancellation: tokio_util::sync::CancellationToken,
    ) -> impl Future<Output = Result<AuthenticatorGetAssertionResponse, FidoClientError>> + Send;
}

/// No internal authenticator is connected by default.
impl InternalAuthenticator for () {
    async fn make_credential(
        &self,
        _: &AuthenticatorMakeCredentialRequest,
        _: tokio_util::sync::CancellationToken,
    ) -> Result<AuthenticatorMakeCredentialResponse, FidoClientError> {
        Err(FidoClientError::NoAuthenticator)
    }

    async fn get_assertion(
        &self,
        _: &AuthenticatorGetAssertionRequest,
        _: tokio_util::sync::CancellationToken,
    ) -> Result<AuthenticatorGetAssertionResponse, FidoClientError> {
        Err(FidoClientError::NoAuthenticator)
    }
}
