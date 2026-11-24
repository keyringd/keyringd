use futures_util::future::BoxFuture;
use keyringd_fido::webauthn::{
    PublicKeyCredentialCreationOptions, PublicKeyCredentialRequestOptions,
};
use keyringd_fido_client::{
    ClientData, FidoClientError, GetAssertionsResult, MakeCredentialResult, OperationId,
    backend::OsUsbBackend,
};
use tokio_util::sync::CancellationToken;

mockall::mock! {
    /// Mock to set the number of requests, responses, and calls to pass to the OS registration/authentication API.
    pub(crate) OsUsbBackend {}
    impl OsUsbBackend for OsUsbBackend {
        fn make_credential(&self, operation_id: OperationId, options: PublicKeyCredentialCreationOptions, client_data: ClientData, cancellation: CancellationToken)
            -> BoxFuture<'static, Result<MakeCredentialResult, FidoClientError>>;
        fn get_assertions(&self, operation_id: OperationId, options: PublicKeyCredentialRequestOptions, client_data: ClientData, cancellation: CancellationToken)
            -> BoxFuture<'static, Result<GetAssertionsResult, FidoClientError>>;
    }
}
