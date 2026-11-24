use super::ctap_transport_error::CtapTransportError;
use futures_util::future::BoxFuture;
use keyringd_fido::{CtapRequest, CtapResponse};

/// A common interface of Transport that provides for sending, canceling, and connecting to CTAP requests.
/// Treat USB and Hybrid as Box <Dyn CtapTransport>. Notification destinations will be set when building a specific channel.
pub trait CtapTransport: Send {
    fn transmit<'a>(
        &'a mut self,
        request: &'a CtapRequest,
    ) -> BoxFuture<'a, Result<CtapResponse, CtapTransportError>>;
    fn cancel(&mut self) -> BoxFuture<'_, Result<(), CtapTransportError>>;
    fn close(&mut self) -> BoxFuture<'_, Result<(), CtapTransportError>> {
        Box::pin(async { Ok(()) })
    }
}
