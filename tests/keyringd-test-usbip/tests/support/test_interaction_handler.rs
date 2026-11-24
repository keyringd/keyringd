use futures_util::future::BoxFuture;
use keyringd_fido::ctap::Pin;
use keyringd_fido_client::NewPinInteraction;
use keyringd_fido_client::*;
use std::sync::Mutex;
#[derive(Default)]
pub struct TestInteractionHandler {
    pub events: Mutex<Vec<OperationStatus>>,
    pub pins: Mutex<usize>,
}
impl FidoInteractionHandler for TestInteractionHandler {
    fn request_transport(
        &self,
        request: keyringd_fido_client::TransportInteraction,
    ) -> futures_util::future::BoxFuture<
        '_,
        Result<
            keyringd_fido_client::InteractionResponse<
                Option<keyringd_fido::AuthenticatorTransport>,
            >,
            keyringd_fido_client::FidoInteractionHandlerError,
        >,
    > {
        Box::pin(async move {
            Ok(keyringd_fido_client::InteractionResponse::new(
                request.id(),
                Some(keyringd_fido::AuthenticatorTransport::usb()),
            ))
        })
    }

    fn on_operation_status_changed(
        &self,
        event: OperationStatus,
    ) -> BoxFuture<'_, Result<(), FidoInteractionHandlerError>> {
        self.events.lock().unwrap().push(event);
        Box::pin(async { Ok(()) })
    }

    fn request_pin(
        &self,
        interaction: PinInteraction,
    ) -> BoxFuture<'_, Result<InteractionResponse<Option<Pin>>, FidoInteractionHandlerError>> {
        *self.pins.lock().unwrap() += 1;
        Box::pin(async move {
            Ok(InteractionResponse::new(
                interaction.id(),
                Some(Pin::from_text("1234").unwrap()),
            ))
        })
    }

    fn request_new_pin(
        &self,
        interaction: NewPinInteraction,
    ) -> BoxFuture<'_, Result<InteractionResponse<Option<Pin>>, FidoInteractionHandlerError>> {
        *self.pins.lock().unwrap() += 1;
        Box::pin(async move {
            Ok(InteractionResponse::new(
                interaction.id(),
                Some(Pin::from_text("1234").unwrap()),
            ))
        })
    }
}
