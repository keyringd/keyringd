use futures_util::future::BoxFuture;
use keyringd_fido_client::{FidoInteractionHandler, FidoInteractionHandlerError, OperationId};

/// Connects the test service to each product's actual QR presentation path.
pub trait TestFidoInteractionHandler: FidoInteractionHandler {
    fn present_hybrid_qr<'a>(
        &'a self,
        operation_id: OperationId,
        uri: &'a str,
    ) -> BoxFuture<'a, Result<(), FidoInteractionHandlerError>>;
}
