use crate::NewPinInteraction;
use futures_util::future::BoxFuture;
use keyringd_fido::ctap::Pin;

use crate::{FidoInteractionHandlerError, InteractionResponse, OperationStatus, PinInteraction};

/// An interactive interface that connects FIDO operations with the UI of the app.
/// Leave the selection of routes, notification of progress, input of existing PINs, and input of new PIN settings to the app.
pub trait FidoInteractionHandler: Send + Sync {
    fn request_transport(
        &self,
        request: crate::TransportInteraction,
    ) -> BoxFuture<
        '_,
        Result<
            InteractionResponse<Option<keyringd_fido::AuthenticatorTransport>>,
            FidoInteractionHandlerError,
        >,
    >;

    fn on_operation_status_changed(
        &self,
        event: OperationStatus,
    ) -> BoxFuture<'_, Result<(), FidoInteractionHandlerError>>;

    fn request_pin(
        &self,
        request: PinInteraction,
    ) -> BoxFuture<'_, Result<InteractionResponse<Option<Pin>>, FidoInteractionHandlerError>>;

    fn request_new_pin(
        &self,
        request: NewPinInteraction,
    ) -> BoxFuture<'_, Result<InteractionResponse<Option<Pin>>, FidoInteractionHandlerError>>;
}

/// Headless use: notifications are accepted; interactive requests are cancelled.
impl FidoInteractionHandler for () {
    fn request_transport(
        &self,
        request: crate::TransportInteraction,
    ) -> BoxFuture<
        '_,
        Result<
            InteractionResponse<Option<keyringd_fido::AuthenticatorTransport>>,
            FidoInteractionHandlerError,
        >,
    > {
        Box::pin(async move { Ok(InteractionResponse::new(request.id(), None)) })
    }
    fn on_operation_status_changed(
        &self,
        _: OperationStatus,
    ) -> BoxFuture<'_, Result<(), FidoInteractionHandlerError>> {
        Box::pin(async { Ok(()) })
    }
    fn request_pin(
        &self,
        request: PinInteraction,
    ) -> BoxFuture<'_, Result<InteractionResponse<Option<Pin>>, FidoInteractionHandlerError>> {
        Box::pin(async move { Ok(InteractionResponse::new(request.id(), None)) })
    }
    fn request_new_pin(
        &self,
        request: NewPinInteraction,
    ) -> BoxFuture<'_, Result<InteractionResponse<Option<Pin>>, FidoInteractionHandlerError>> {
        Box::pin(async move { Ok(InteractionResponse::new(request.id(), None)) })
    }
}

impl<T> FidoInteractionHandler for std::sync::Arc<T>
where
    T: FidoInteractionHandler + ?Sized,
{
    fn request_transport(
        &self,
        request: crate::TransportInteraction,
    ) -> BoxFuture<
        '_,
        Result<
            InteractionResponse<Option<keyringd_fido::AuthenticatorTransport>>,
            FidoInteractionHandlerError,
        >,
    > {
        (**self).request_transport(request)
    }

    fn on_operation_status_changed(
        &self,
        event: OperationStatus,
    ) -> BoxFuture<'_, Result<(), FidoInteractionHandlerError>> {
        (**self).on_operation_status_changed(event)
    }

    fn request_pin(
        &self,
        request: PinInteraction,
    ) -> BoxFuture<'_, Result<InteractionResponse<Option<Pin>>, FidoInteractionHandlerError>> {
        (**self).request_pin(request)
    }

    fn request_new_pin(
        &self,
        request: NewPinInteraction,
    ) -> BoxFuture<'_, Result<InteractionResponse<Option<Pin>>, FidoInteractionHandlerError>> {
        (**self).request_new_pin(request)
    }
}
