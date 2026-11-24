use crate::{FidoClientError, InternalAuthenticator};
use keyringd_fido::ctap::{
    AuthenticatorGetAssertionRequest, AuthenticatorGetAssertionResponse,
    AuthenticatorMakeCredentialRequest, AuthenticatorMakeCredentialResponse,
};
mockall::mock! {
    pub(crate) InternalAuthenticator {}
    impl InternalAuthenticator for InternalAuthenticator {
        fn make_credential(&self, request: &AuthenticatorMakeCredentialRequest, cancellation: tokio_util::sync::CancellationToken) -> impl std::future::Future<Output = Result<AuthenticatorMakeCredentialResponse, FidoClientError>> + Send;
        fn get_assertion(&self, request: &AuthenticatorGetAssertionRequest, cancellation: tokio_util::sync::CancellationToken) -> impl std::future::Future<Output = Result<AuthenticatorGetAssertionResponse, FidoClientError>> + Send;
    }
}
