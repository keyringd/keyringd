use crate::app::ui::pin_prompt_window::PinPromptWindow;
use keyringd_core::application::ActionId;
use keyringd_ui_core::runtime::UiEventSender;
use std::rc::Rc;
use std::sync::Arc;

use objc2::MainThreadMarker;
use objc2::rc::Retained;

use crate::app::ui::AppShell;
use crate::app::ui::qr_prompt_window::QrPromptWindow;
use crate::app::ui::security_key_window::SecurityKeyWindow;
use keyringd_ui_core::key_operation::{
    KeyOperationCommand, KeyOperationCoordinator, KeyOperationEvent,
};
use keyringd_ui_core::qr_prompt_window::HybridQrPresentation;

pub(crate) struct AuthenticationPromptCommandHandler {
    mtm: MainThreadMarker,
    shell: Retained<AppShell>,
    interaction: Arc<KeyOperationCoordinator>,
    flow_id: Option<ActionId>,
    inputs: UiEventSender<(ActionId, KeyOperationEvent)>,
    hybrid: QrPromptWindow,
    fido: SecurityKeyWindow,
    pin: Option<Rc<PinPromptWindow>>,
}

impl AuthenticationPromptCommandHandler {
    pub(crate) fn new(
        mtm: MainThreadMarker,
        interaction: Arc<KeyOperationCoordinator>,
        flow_id: Option<ActionId>,
        shell: Retained<AppShell>,
        inputs: UiEventSender<(ActionId, KeyOperationEvent)>,
    ) -> Self {
        Self {
            mtm,
            shell: shell.clone(),
            interaction,
            flow_id,
            inputs,
            hybrid: QrPromptWindow::new(mtm, shell.clone()),
            fido: SecurityKeyWindow::new(mtm, shell),
            pin: None,
        }
    }

    pub(crate) fn accepts(&self, id: ActionId) -> bool {
        self.interaction.accepts_input(self.flow_id, id)
    }

    fn events(&self, id: ActionId) -> Option<UiEventSender<KeyOperationEvent>> {
        if !self.accepts(id) {
            return None;
        }
        Some(self.inputs.map(move |event| (id, event)))
    }

    pub(crate) fn handle(&mut self, command: KeyOperationCommand, source_flow: ActionId) {
        let span = self
            .interaction
            .log_span(source_flow, None, command.interaction_id());
        let _entered = span.enter();
        tracing::debug!("processing authentication UI command");
        match command {
            KeyOperationCommand::CancelOperation => {
                self.interaction.cancel_action(source_flow);
            }
            KeyOperationCommand::PresentHybridQr {
                prompt_id,
                value,
                context,
                unavailable,
                security_key_available,
                presented,
                ..
            } => {
                let Some(context) = context else {
                    return;
                };
                let presentation =
                    HybridQrPresentation::new(value, &context, security_key_available, unavailable);
                let Some(events) = self.events(source_flow) else {
                    return;
                };
                self.hybrid
                    .present(presentation, prompt_id, presented, events);
            }
            KeyOperationCommand::UpdateHybridAvailability { reason } => {
                self.hybrid.show_unavailable(reason);
            }
            KeyOperationCommand::RequestPin {
                request_id,
                reason,
                attempts_left,
                minimum_length,
                new_pin,
                response,
            } => self.request_pin(
                request_id,
                keyringd_ui_core::pin_prompt_window::SecurityKeyPinPrompt {
                    reason,
                    attempts_left,
                    minimum_length,
                    new_pin,
                },
                response,
                source_flow,
            ),
            KeyOperationCommand::AnswerPin { value, response } => {
                let _ = response.send(value);
            }
            KeyOperationCommand::ShowAuthenticatorPrompt {
                transport,
                operation,
                prompt_kind,
                ..
            } => {
                if transport == keyringd_fido::AuthenticatorTransport::usb()
                    && let Some(events) = self.events(source_flow)
                {
                    self.dismiss_pin();
                    self.fido
                        .show_security_key_prompt(&operation, prompt_kind, false, events);
                }
            }
            KeyOperationCommand::DismissHybridQr { acknowledged, .. } => {
                self.hybrid.dismiss();
                let _ = acknowledged.send(());
            }
            KeyOperationCommand::CloseOperationStatus => {
                self.hybrid.dismiss();
                self.fido.dismiss();
                self.dismiss_pin();
            }
            KeyOperationCommand::UpdateOperation { .. } => {
                self.fido.dismiss();
                self.dismiss_pin();
            }
            KeyOperationCommand::ActivateUsb(operation_id) => {
                let activated = self
                    .interaction
                    .fido_operations
                    .activate_usb(source_flow, operation_id);
                if activated {
                    self.hybrid.usb_selected();
                } else {
                    tracing::debug!("USB activation was rejected — no active flow");
                }
            }
            KeyOperationCommand::AcknowledgeStaleHybridDismissal(acknowledged) => {
                let _ = acknowledged.send(());
            }
            KeyOperationCommand::RequestDeviceKeyPassword { .. }
            | KeyOperationCommand::RequestNewDeviceKeyPassword { .. }
            | KeyOperationCommand::RequestDeviceKeyPasswordChange { .. } => {
                // Dropping the response reports a failed request rather than user cancellation.
                tracing::error!("unexpected app-managed Device Key password request on macOS");
            }
            KeyOperationCommand::AnswerDeviceKeyPassword { value, response } => {
                let _ = response.send(value);
            }
            KeyOperationCommand::AnswerDeviceKeyPasswordChange { value, response } => {
                let _ = response.send(value);
            }
        }
    }

    fn request_pin(
        &mut self,
        request_id: keyringd_fido_client::InteractionId,
        prompt: keyringd_ui_core::pin_prompt_window::SecurityKeyPinPrompt,
        response: tokio::sync::oneshot::Sender<Option<String>>,
        source_flow: ActionId,
    ) {
        let Some(events) = self.events(source_flow) else {
            let _ = response.send(None);
            return;
        };
        self.fido.dismiss();
        self.dismiss_pin();
        self.pin = Some(PinPromptWindow::present(
            self.mtm,
            self.shell.clone(),
            prompt,
            request_id,
            response,
            events,
        ));
    }

    fn dismiss_pin(&mut self) {
        if let Some(pin) = self.pin.take() {
            pin.dismiss();
        }
    }
}
