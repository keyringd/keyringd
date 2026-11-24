use crate::*;
use crate::{FidoInteractionHandlerError, InteractionResponse};

use keyringd_fido::AuthenticatorTransport;
use keyringd_fido::ctap::Pin;

use std::sync::{Arc, Mutex};

use crate::NewPinInteraction;

/// A test handler that records interaction requests and progress and controls PIN input, route selection, and cancellation responses.
#[derive(Default)]
pub(crate) struct RecordingHandler {
    pub(crate) fail_completion: bool,
    pub(crate) transport_response: Option<AuthenticatorTransport>,
    pub(crate) transport_requests: Mutex<Vec<crate::TransportInteraction>>,
    pub(crate) transport_prompt_gate: Option<Arc<tokio::sync::Notify>>,
    pub(crate) events: Mutex<Vec<&'static str>>,
    pub(crate) qr_uris: Mutex<Vec<String>>,
    pub(crate) hybrid_unavailable: Mutex<Vec<crate::HybridTransportUnavailableReason>>,
    pub(crate) existing_pin_requests: Mutex<Vec<PinInteraction>>,
    pub(crate) new_pin_requests: Mutex<Vec<NewPinInteraction>>,
    pub(crate) pin: Option<&'static str>,
}

impl FidoInteractionHandler for RecordingHandler {
    fn request_transport(
        &self,
        request: crate::TransportInteraction,
    ) -> futures_util::future::BoxFuture<
        '_,
        Result<
            crate::InteractionResponse<Option<keyringd_fido::AuthenticatorTransport>>,
            crate::FidoInteractionHandlerError,
        >,
    > {
        self.transport_requests.lock().unwrap().push(request);
        Box::pin(async move {
            if let Some(gate) = &self.transport_prompt_gate {
                gate.notified().await;
            }
            Ok(crate::InteractionResponse::new(
                request.id(),
                self.transport_response.clone(),
            ))
        })
    }

    fn on_operation_status_changed(
        &self,
        event: OperationStatus,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<(), FidoInteractionHandlerError>> + Send + '_>,
    > {
        Box::pin(async move {
            let fail =
                self.fail_completion && matches!(event, OperationStatus::BackendFinished { .. });
            if let OperationStatus::HybridQrReady { qr_uri, .. } = &event {
                self.qr_uris.lock().unwrap().push(qr_uri.clone());
            }
            if let OperationStatus::HybridTransportUnavailable { reason, .. } = &event {
                self.hybrid_unavailable.lock().unwrap().push(*reason);
            }
            self.events.lock().unwrap().push(match event {
                OperationStatus::AuthenticatorSelectionRequired { .. } => "selection",
                OperationStatus::OperationStarted { .. } => "started",
                OperationStatus::BackendFinished { .. } => "backend_finished",
                _ => "progress",
            });
            if fail {
                Err(FidoInteractionHandlerError::new(std::io::Error::other(
                    "completion notification failed",
                )))
            } else {
                Ok(())
            }
        })
    }

    fn request_pin(
        &self,
        request: PinInteraction,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<
                    Output = Result<InteractionResponse<Option<Pin>>, FidoInteractionHandlerError>,
                > + Send
                + '_,
        >,
    > {
        self.existing_pin_requests.lock().unwrap().push(request);
        let pin = self.pin.map(|value| Pin::from_text(value).unwrap());
        Box::pin(async move { Ok(InteractionResponse::new(request.id(), pin)) })
    }

    fn request_new_pin(
        &self,
        request: NewPinInteraction,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<
                    Output = Result<InteractionResponse<Option<Pin>>, FidoInteractionHandlerError>,
                > + Send
                + '_,
        >,
    > {
        self.new_pin_requests.lock().unwrap().push(request);
        let pin = self
            .pin
            .map(|value| Pin::from_text_with_minimum(value, request.minimum_length()).unwrap());
        Box::pin(async move { Ok(InteractionResponse::new(request.id(), pin)) })
    }
}
