use gtk4::prelude::{GtkWindowExt, ObjectExt, WidgetExt};
use keyringd_core::application::ActionId;

use keyringd_ui_core::key_operation::AuthenticatorPromptKind;
use keyringd_ui_core::key_operation::{
    Action, KeyOperationCoordinator, KeyOperationEvent, KeyOperationState,
};
use keyringd_ui_core::{ComponentModel, runtime::UiEventSender};
use std::sync::Arc;

use crate::app::ui::device_key_password_window::DeviceKeyPasswordWindow;
use crate::app::ui::qr_prompt_window::HybridQrView;
use crate::app::ui::qr_prompt_window::QrPromptWindow;
use crate::app::ui::security_key_window::SecurityKeyPromptWindow;
use keyringd_ui_core::key_operation::KeyOperationCommand;
use keyringd_ui_core::pin_prompt_window::SecurityKeyPinPrompt;
use keyringd_ui_core::qr_prompt_window::HybridQrPresentation;

pub(in crate::app) struct AuthenticationPromptCommandHandler {
    application: libadwaita::Application,
    owner: Option<gtk4::Window>,
    interaction: Arc<KeyOperationCoordinator>,
    flow_id: Option<ActionId>,
    events: keyringd_ui_core::runtime::UiEventSender<(ActionId, KeyOperationEvent)>,
    operation_status: Option<SecurityKeyPromptWindow>,
    pin_prompt: Option<crate::app::ui::pin_prompt_window::PinPromptWindow>,
    device_key_password_prompt: Option<libadwaita::MessageDialog>,
    active_hybrid: Option<(QrPromptWindow, HybridQrView)>,
}

impl AuthenticationPromptCommandHandler {
    pub(in crate::app) fn new(
        application: libadwaita::Application,
        owner: Option<gtk4::Window>,
        interaction: Arc<KeyOperationCoordinator>,
        flow_id: Option<ActionId>,
        events: keyringd_ui_core::runtime::UiEventSender<(ActionId, KeyOperationEvent)>,
    ) -> Self {
        Self {
            application,
            owner,
            interaction,
            flow_id,
            events,
            operation_status: None,
            pin_prompt: None,
            device_key_password_prompt: None,
            active_hybrid: None,
        }
    }

    /// Runs backend work while handling its authentication prompts on the GTK thread.
    pub(in crate::app) async fn run_action<T: Send + 'static>(
        application: libadwaita::Application,
        owner: gtk4::Window,
        interaction: Arc<KeyOperationCoordinator>,
        mut action: Action,
        operation: impl std::future::Future<Output = T> + Send + 'static,
    ) -> Option<T> {
        let id = action.id();
        let mut key_events = action.take_events();
        let cancellation = action.cancellation_token();
        let _cancellation_guard = cancellation.clone().drop_guard();
        let (ui_events, mut ui_receiver) = UiEventSender::channel();
        let destroyed = ui_events.clone();
        let destroy_handler = owner
            .connect_destroy(move |_| destroyed.send((id, KeyOperationEvent::CancelRequested)));
        let mut handler = Self::new(
            application,
            Some(owner.clone()),
            interaction,
            Some(id),
            ui_events,
        );
        let (finished, mut finished_receiver) = UiEventSender::channel();
        tokio::spawn(async move {
            finished.send(action.run(operation).await);
        });

        let mut state = KeyOperationState::default();
        let mut backend_open = true;
        let result = loop {
            let event = tokio::select! {
                finished = finished_receiver.receive() => break finished,
                request = key_events.receive(), if backend_open => match request {
                    Some((source, event)) => {
                        if source != id { event.reject(); continue; }
                        event
                    }
                    None => {
                        backend_open = false;
                        cancellation.cancel();
                        continue;
                    }
                },
                Some((source, event)) = ui_receiver.receive() => {
                    if source != id || !handler.accepts(source) {
                        event.reject();
                        continue;
                    }
                    event
                }
            };
            // Cancellation must still allow the backend to finish dismissing its prompts.
            if cancellation.is_cancelled()
                && !matches!(
                    &event,
                    KeyOperationEvent::HybridQrDismissed { .. }
                        | KeyOperationEvent::PromptDismissalRequested
                )
            {
                event.reject();
                continue;
            }
            if let Some(command) = state.update(event) {
                handler.handle(command, id);
            }
        };
        owner.disconnect(destroy_handler);
        // Drop closes all prompts before the caller handles the operation's result.
        drop(handler);
        result
    }

    pub(in crate::app) fn reset(&mut self) {
        self.close_operation_prompts();
        self.dismiss_hybrid();
    }
    pub(in crate::app) fn accepts(&self, id: ActionId) -> bool {
        self.interaction.accepts_input(self.flow_id, id)
    }

    fn prompt_events(
        &self,
        flow_id: ActionId,
    ) -> keyringd_ui_core::runtime::UiEventSender<KeyOperationEvent> {
        self.events.map(move |event| (flow_id, event))
    }

    pub(in crate::app) fn handle(&mut self, command: KeyOperationCommand, target_flow: ActionId) {
        let span = self
            .interaction
            .log_span(target_flow, None, command.interaction_id());
        let _entered = span.enter();
        tracing::debug!("processing authentication UI command");
        match command {
            KeyOperationCommand::CancelOperation => {
                self.interaction.cancel_action(target_flow);
            }
            KeyOperationCommand::UpdateOperation { .. } => {}
            KeyOperationCommand::PresentHybridQr {
                prompt_id,
                value,
                context,
                security_key_available,
                presented,
                replace_active,
                unavailable,
            } => {
                if replace_active {
                    self.dismiss_hybrid();
                }
                let Some(context) = context else {
                    return;
                };
                let presentation =
                    HybridQrPresentation::new(value, &context, security_key_available, unavailable);
                let view = HybridQrView::new(&presentation);
                let dialog = QrPromptWindow::present(
                    &self.application,
                    self.owner.as_ref(),
                    &view.widget(),
                    &presentation,
                    prompt_id,
                    self.prompt_events(target_flow),
                );
                self.active_hybrid = Some((dialog, view));
                let _ = presented.send(());
            }
            KeyOperationCommand::UpdateHybridAvailability { reason } => {
                if let Some((_, view)) = &self.active_hybrid {
                    view.show_unavailable(reason);
                }
            }
            KeyOperationCommand::DismissHybridQr { acknowledged, .. } => {
                self.dismiss_hybrid();
                let _ = acknowledged.send(());
            }
            KeyOperationCommand::AcknowledgeStaleHybridDismissal(acknowledged) => {
                tracing::debug!("ignoring stale Hybrid prompt dismissal");
                let _ = acknowledged.send(());
            }
            KeyOperationCommand::ShowAuthenticatorPrompt {
                operation,
                transport: _,
                prompt_kind,
            } => self.show_security_key_prompt(operation, prompt_kind, target_flow),
            KeyOperationCommand::CloseOperationStatus => self.close_operation_prompts(),
            KeyOperationCommand::RequestPin {
                request_id,
                reason,
                attempts_left,
                minimum_length,
                new_pin,
                response,
            } => {
                self.close_operation_prompts();
                let prompt = SecurityKeyPinPrompt {
                    reason,
                    attempts_left,
                    minimum_length,
                    new_pin,
                };
                self.pin_prompt =
                    Some(crate::app::ui::pin_prompt_window::PinPromptWindow::present(
                        &self.application,
                        self.owner.as_ref(),
                        &prompt,
                        request_id,
                        response,
                        self.prompt_events(target_flow),
                    ));
            }
            KeyOperationCommand::AnswerPin { value, response } => {
                self.close_operation_prompts();
                let _ = response.send(value);
            }
            KeyOperationCommand::ActivateUsb(operation_id) => {
                if self
                    .interaction
                    .fido_operations
                    .activate_usb(target_flow, operation_id)
                    && let Some((dialog, _)) = &self.active_hybrid
                {
                    dialog.usb_selected();
                }
            }

            KeyOperationCommand::RequestDeviceKeyPassword {
                profile_name,
                response,
            } => {
                self.close_device_key_password_prompt();
                self.device_key_password_prompt = Some(DeviceKeyPasswordWindow::request_password(
                    &self.application,
                    self.owner.as_ref(),
                    &profile_name,
                    false,
                    response,
                    self.prompt_events(target_flow),
                ));
            }
            KeyOperationCommand::RequestNewDeviceKeyPassword {
                profile_name,
                response,
            } => {
                self.close_device_key_password_prompt();
                self.device_key_password_prompt = Some(DeviceKeyPasswordWindow::request_password(
                    &self.application,
                    self.owner.as_ref(),
                    &profile_name,
                    true,
                    response,
                    self.prompt_events(target_flow),
                ));
            }
            KeyOperationCommand::RequestDeviceKeyPasswordChange {
                profile_name,
                change,
                response,
            } => {
                self.close_device_key_password_prompt();
                self.device_key_password_prompt =
                    Some(DeviceKeyPasswordWindow::request_password_change(
                        &self.application,
                        self.owner.as_ref(),
                        &profile_name,
                        change,
                        response,
                        self.prompt_events(target_flow),
                    ));
            }
            KeyOperationCommand::AnswerDeviceKeyPassword { value, response } => {
                self.close_device_key_password_prompt();
                let _ = response.send(value);
            }
            KeyOperationCommand::AnswerDeviceKeyPasswordChange { value, response } => {
                self.close_device_key_password_prompt();
                let _ = response.send(value);
            }
        }
    }

    fn show_security_key_prompt(
        &mut self,
        operation: keyringd_ui_core::key_operation::KeyOperationKind,
        prompt_kind: AuthenticatorPromptKind,
        target_flow: ActionId,
    ) {
        self.close_operation_prompts();
        self.operation_status = Some(SecurityKeyPromptWindow::new(
            &self.application,
            self.owner.as_ref(),
            &operation,
            prompt_kind,
            self.prompt_events(target_flow),
        ));
    }

    fn close_operation_status(&mut self) {
        if let Some(status) = self.operation_status.take() {
            status.close();
        }
    }

    fn close_operation_prompts(&mut self) {
        self.close_operation_status();
        if let Some(prompt) = self.pin_prompt.take() {
            prompt.close();
        }
        self.close_device_key_password_prompt();
    }

    fn close_device_key_password_prompt(&mut self) {
        if let Some(prompt) = self.device_key_password_prompt.take() {
            prompt.close();
        }
    }

    fn dismiss_hybrid(&mut self) {
        if let Some((dialog, _)) = self.active_hybrid.take() {
            // Closing a completed presentation must not cancel its operation.
            dialog.dismiss();
        }
    }
}

impl Drop for AuthenticationPromptCommandHandler {
    fn drop(&mut self) {
        self.close_operation_prompts();
        self.dismiss_hybrid();
    }
}
