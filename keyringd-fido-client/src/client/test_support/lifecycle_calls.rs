use crate::{InteractionResponse, OperationId};
use keyringd_fido::AuthenticatorTransport;
use std::sync::{Arc, Mutex};

/// A shared record of the test side that records notifications such as the start/end of operations and verifies the life management of the operation.
#[derive(Default)]
pub(crate) struct LifecycleCalls {
    pub(crate) events: Mutex<Vec<(&'static str, OperationId)>>,
}

pub(crate) fn lifecycle_handler(
    calls: Arc<LifecycleCalls>,
) -> crate::test_support::MockFidoInteractionHandler {
    let mut handler = crate::test_support::MockFidoInteractionHandler::new();
    handler.expect_request_transport().returning(|request| {
        Box::pin(async move {
            Ok(InteractionResponse::new(
                request.id(),
                Some(AuthenticatorTransport::usb()),
            ))
        })
    });
    handler
        .expect_on_operation_status_changed()
        .returning(move |event| {
            let calls = calls.clone();
            Box::pin(async move {
                let name = match event {
                    crate::OperationStatus::OperationStarted { .. } => "started",
                    crate::OperationStatus::BackendFinished { .. } => "backend_finished",
                    _ => "progress",
                };
                calls
                    .events
                    .lock()
                    .unwrap()
                    .push((name, event.operation_id()));
                Ok(())
            })
        });
    handler.expect_request_pin().returning(|request| {
        Box::pin(async move { Ok(InteractionResponse::new(request.id(), None)) })
    });
    handler.expect_request_new_pin().returning(|request| {
        Box::pin(async move { Ok(InteractionResponse::new(request.id(), None)) })
    });
    handler
}
