use crate::app::AuthenticationPromptWindows;
use crate::app::ui::pin_prompt_window::PinPromptWindow;
use crate::app::ui::qr_prompt_window::QrPromptWindow;
use keyringd_core::application::ActionId;
use keyringd_fido_client::OperationId;
use keyringd_ui_core::key_operation::{KeyOperationCommand, KeyOperationCoordinator};
use keyringd_ui_core::pin_prompt_window::SecurityKeyPinPrompt;
use keyringd_ui_core::qr_prompt_window::HybridQrPresentation;
use std::sync::Arc;
use windows_reactor::UiMarshaller;

/// Creates native prompts and drives the authentication state of each action.
#[derive(Clone)]
pub(crate) struct AuthenticationPromptCommandHandler {
    pub(crate) dispatcher: UiMarshaller,
    pub(crate) runtime: tokio::runtime::Handle,
    pub(crate) coordinator: Arc<KeyOperationCoordinator>,
}
impl AuthenticationPromptCommandHandler {
    pub(crate) fn new(
        dispatcher: UiMarshaller,
        runtime: tokio::runtime::Handle,
        coordinator: Arc<KeyOperationCoordinator>,
    ) -> Self {
        Self {
            dispatcher,
            runtime,
            coordinator,
        }
    }
    pub(crate) fn handle(
        &self,
        flow: ActionId,
        session: &AuthenticationPromptWindows,
        operation_id: Option<OperationId>,
        command: KeyOperationCommand,
    ) {
        let span = self
            .coordinator
            .log_span(flow, operation_id, command.interaction_id());
        let _entered = span.enter();
        tracing::debug!("processing authentication UI command");
        let session = session.clone();
        match command {
            KeyOperationCommand::PresentHybridQr {
                prompt_id,
                value,
                context,
                security_key_available,
                presented,
                unavailable,
                ..
            } => {
                session.show_prompt(prompt_id);
                session.show_hybrid_availability(prompt_id, unavailable);
                let Some(operation_id) = operation_id else {
                    return;
                };
                session.qr_opened(prompt_id);
                let failure_session = session.clone();
                let Some(context) = context else {
                    return;
                };
                let presentation =
                    HybridQrPresentation::new(value, &context, security_key_available, unavailable);
                let prompts = self.clone();
                let log_span = tracing::Span::current();
                self.dispatcher.dispatch(move || {
                    log_span.in_scope(|| {
                        if !session.is_active(prompt_id) {
                            failure_session.qr_closed(prompt_id);
                            return;
                        }
                        match QrPromptWindow::open(
                            operation_id,
                            flow,
                            presentation,
                            prompts,
                            session,
                            prompt_id,
                        ) {
                            Ok(()) => {
                                let _ = presented.send(());
                            }
                            Err(error) => {
                                failure_session.qr_closed(prompt_id);
                                tracing::error!(%error, "could not open QR prompt");
                            }
                        }
                    })
                });
            }
            KeyOperationCommand::DismissHybridQr {
                prompt_id,
                acknowledged,
            } => {
                session.dismiss_hybrid(prompt_id, acknowledged);
            }
            KeyOperationCommand::AcknowledgeStaleHybridDismissal(acknowledged) => {
                let _ = acknowledged.send(());
            }
            KeyOperationCommand::RequestPin {
                request_id,
                reason,
                attempts_left,
                minimum_length,
                new_pin,
                response,
            } => {
                let prompt = SecurityKeyPinPrompt {
                    reason,
                    attempts_left,
                    minimum_length,
                    new_pin,
                };
                session.show_prompt(request_id);
                let runtime = self.runtime.clone();
                let Some(events) = self.coordinator.event_sender(flow) else {
                    return;
                };
                let log_span = tracing::Span::current();
                self.dispatcher.dispatch(move || {
                    log_span.in_scope(|| {
                        if !session.is_active(request_id) {
                            events.send(
                                keyringd_ui_core::key_operation::KeyOperationEvent::PinAnswered {
                                    request_id,
                                    value: None,
                                    response,
                                },
                            );
                            return;
                        }
                        if let Err(error) = PinPromptWindow::open(
                            prompt, response, runtime, session, request_id, events,
                        ) {
                            tracing::error!(%error, "could not open PIN prompt");
                        }
                    })
                });
            }
            KeyOperationCommand::UpdateHybridAvailability { reason } => {
                session.hybrid_unavailable(reason);
            }
            KeyOperationCommand::UpdateOperation { .. }
            | KeyOperationCommand::ShowAuthenticatorPrompt { .. } => {}
            KeyOperationCommand::CloseOperationStatus => {
                session.dismiss();
            }
            KeyOperationCommand::CancelOperation => self.coordinator.cancel_action(flow),
            KeyOperationCommand::ActivateUsb(operation_id) => {
                if self
                    .coordinator
                    .fido_operations
                    .activate_usb(flow, operation_id)
                {
                    session.dismiss();
                }
            }
            KeyOperationCommand::AnswerPin { value, response } => {
                let _ = response.send(value);
            }
            KeyOperationCommand::RequestDeviceKeyPassword { .. }
            | KeyOperationCommand::RequestNewDeviceKeyPassword { .. }
            | KeyOperationCommand::RequestDeviceKeyPasswordChange { .. } => {
                // Dropping the response reports a failed request rather than user cancellation.
                tracing::error!("unexpected app-managed Device Key password request on Windows");
            }
            KeyOperationCommand::AnswerDeviceKeyPassword { value, response } => {
                let _ = response.send(value);
            }
            KeyOperationCommand::AnswerDeviceKeyPasswordChange { value, response } => {
                let _ = response.send(value);
            }
        }
    }
    pub(crate) async fn receive_events(
        self,
        action_id: ActionId,
        mut events: keyringd_ui_core::runtime::UiEventReceiver<(
            ActionId,
            keyringd_ui_core::key_operation::KeyOperationEvent,
        )>,
    ) {
        use keyringd_ui_core::{
            ComponentModel,
            key_operation::{KeyOperationEvent, KeyOperationState},
        };
        let session = crate::app::AuthenticationPromptWindows::new();
        let mut state = KeyOperationState::default();
        while let Some((source, event)) = events.receive().await {
            if source != action_id
                || (!self.coordinator.accepts_input(Some(action_id), source)
                    && !matches!(
                        &event,
                        KeyOperationEvent::PromptDismissalRequested
                            | KeyOperationEvent::HybridQrDismissed { .. }
                    ))
            {
                event.reject();
                continue;
            }
            let operation = match &event {
                KeyOperationEvent::HybridQrRequested { prompt_id, .. }
                | KeyOperationEvent::HybridQrDismissed { prompt_id, .. } => {
                    Some(prompt_id.operation_id())
                }
                _ => self.coordinator.fido_operations.active_operation(action_id),
            };
            if let Some(command) = state.update(event) {
                self.handle(action_id, &session, operation, command);
            }
        }
        session.dismiss();
    }
}
