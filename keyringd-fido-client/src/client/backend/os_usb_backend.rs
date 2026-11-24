use keyringd_fido::webauthn::{
    PublicKeyCredentialCreationOptions, PublicKeyCredentialRequestOptions,
};

use crate::{ClientData, FidoClientError, GetAssertionsResult, MakeCredentialResult, OperationId};
use futures_util::future::BoxFuture;
use tokio_util::sync::CancellationToken;

/// An interface for delegating registration, authentication, and cancellation to the OS's USB authentication device API that does not expose raw CTAP communication.
/// The cancel token will be bridged to the OS cancellation API, and will be in charge of cleaning up until the end of the process.
/// It is used to connect Windows WebAuthn API, and another backend is responsible for direct communication by HID.
pub trait OsUsbBackend: Send + Sync {
    fn make_credential(
        &self,
        operation_id: OperationId,
        options: PublicKeyCredentialCreationOptions,
        client_data: ClientData,
        cancellation: CancellationToken,
    ) -> BoxFuture<'_, Result<MakeCredentialResult, FidoClientError>>;

    fn get_assertions(
        &self,
        operation_id: OperationId,
        options: PublicKeyCredentialRequestOptions,
        client_data: ClientData,
        cancellation: CancellationToken,
    ) -> BoxFuture<'_, Result<GetAssertionsResult, FidoClientError>>;
}
