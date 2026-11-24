use crate::app::AuthenticationPromptCommandHandler;
use crate::app::ui::{AppShell, ComponentRuntime};
use keyringd_core::application::ActionId;
use keyringd_ui_core::key_operation::{
    KeyOperationCommand, KeyOperationCoordinator, KeyOperationEvent, KeyOperationState,
};
use keyringd_ui_core::runtime::{UiEventReceiver, UiEventSender};
use objc2::{MainThreadMarker, rc::Retained};
use std::sync::Arc;

pub(crate) struct StandaloneKeyOperationPrompts {
    runtime: ComponentRuntime<KeyOperationState>,
    command_handler: AuthenticationPromptCommandHandler,
    displayed_flow: Option<ActionId>,
}
impl StandaloneKeyOperationPrompts {
    pub(crate) fn new(
        mtm: MainThreadMarker,
        interaction: Arc<KeyOperationCoordinator>,
        shell: Retained<AppShell>,
    ) -> (Self, UiEventReceiver<(ActionId, KeyOperationEvent)>) {
        let (inputs, receiver) = UiEventSender::channel();
        (
            Self {
                runtime: ComponentRuntime::new(KeyOperationState::default()),
                command_handler: AuthenticationPromptCommandHandler::new(
                    mtm,
                    interaction,
                    None,
                    shell,
                    inputs,
                ),
                displayed_flow: None,
            },
            receiver,
        )
    }
    pub(crate) fn dispatch_input(&mut self, flow_id: ActionId, event: KeyOperationEvent) {
        if self.displayed_flow != Some(flow_id) || !self.command_handler.accepts(flow_id) {
            event.reject();
            return;
        }
        if let Some(command) = self.runtime.dispatch(event) {
            self.command_handler.handle(command, flow_id);
        }
    }

    pub(crate) fn dispatch(&mut self, flow_id: ActionId, event: KeyOperationEvent) {
        let active = self.command_handler.accepts(flow_id);
        let terminal = matches!(
            &event,
            KeyOperationEvent::PromptDismissalRequested
                | KeyOperationEvent::HybridQrDismissed { .. }
        );
        if !active && (self.displayed_flow != Some(flow_id) || !terminal) {
            event.reject();
            return;
        }
        if self.displayed_flow != Some(flow_id) {
            if let Some(previous) = self.displayed_flow {
                self.command_handler
                    .handle(KeyOperationCommand::CloseOperationStatus, previous);
            }
            self.runtime = ComponentRuntime::new(KeyOperationState::default());
            self.displayed_flow = Some(flow_id);
        }
        if let Some(command) = self.runtime.dispatch(event) {
            self.command_handler.handle(command, flow_id);
        }
    }
}
