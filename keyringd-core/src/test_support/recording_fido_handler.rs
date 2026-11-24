use super::TestFidoInteractionHandler;
use futures_util::future::BoxFuture;
use keyringd_fido::ctap::Pin;
use keyringd_fido_client::NewPinInteraction;
use keyringd_fido_client::{
    FidoInteractionHandler, FidoInteractionHandlerError, HybridTransportUnavailableReason,
    InteractionResponse, OperationId, OperationStatus, PinInteraction,
};
use std::sync::Mutex;

#[derive(Default)]
pub(super) struct RecordingFidoHandler {
    pub(super) events: Mutex<Vec<&'static str>>,
    pub(super) uris: Mutex<Vec<String>>,
}

impl FidoInteractionHandler for RecordingFidoHandler {
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
        Box::pin(async move {
            let name = match event {
                OperationStatus::OperationStarted { .. } => "started",
                OperationStatus::Processing { .. } => "processing",
                OperationStatus::HybridTransportUnavailable {
                    reason: HybridTransportUnavailableReason::BluetoothDisabled,
                    ..
                } => "bluetooth-disabled",
                OperationStatus::HybridTransportUnavailable {
                    reason: HybridTransportUnavailableReason::NoBluetoothAdapter,
                    ..
                } => "no-adapter",
                OperationStatus::BackendFinished { .. } => "backend_finished",
                _ => panic!("unexpected mock notification"),
            };
            self.events.lock().unwrap().push(name);
            Ok(())
        })
    }

    fn request_pin(
        &self,
        _: PinInteraction,
    ) -> BoxFuture<'_, Result<InteractionResponse<Option<Pin>>, FidoInteractionHandlerError>> {
        Box::pin(async { panic!("mock does not ask for PIN") })
    }

    fn request_new_pin(
        &self,
        _: NewPinInteraction,
    ) -> BoxFuture<'_, Result<InteractionResponse<Option<Pin>>, FidoInteractionHandlerError>> {
        Box::pin(async { panic!("mock does not ask for PIN") })
    }
}

impl TestFidoInteractionHandler for RecordingFidoHandler {
    fn present_hybrid_qr<'a>(
        &'a self,
        _: OperationId,
        uri: &'a str,
    ) -> BoxFuture<'a, Result<(), FidoInteractionHandlerError>> {
        Box::pin(async move {
            self.uris.lock().unwrap().push(uri.to_owned());
            Ok(())
        })
    }
}
