use crate::app::AuthenticationPromptCommandHandler;
use crate::app::AuthenticationPromptWindows;
use keyringd_core::application::ActionId;
use keyringd_ui_core::runtime::UiEventReceiver;
use keyringd_ui_core::{
    ComponentModel,
    key_operation::{KeyOperationEvent, KeyOperationState},
};
use std::collections::HashMap;

/// Presents authentication initiated by desktop/SSH requests without a parent window.
pub(crate) struct StandaloneKeyOperationPrompts {
    prompts: AuthenticationPromptCommandHandler,
    events: UiEventReceiver<(ActionId, KeyOperationEvent)>,
    sessions: HashMap<ActionId, (KeyOperationState, AuthenticationPromptWindows)>,
}
impl StandaloneKeyOperationPrompts {
    pub(crate) fn new(
        prompts: AuthenticationPromptCommandHandler,
        events: UiEventReceiver<(ActionId, KeyOperationEvent)>,
    ) -> Self {
        Self {
            prompts,
            events,
            sessions: HashMap::new(),
        }
    }
    pub(crate) async fn run(mut self) {
        while let Some((flow, event)) = self.events.receive().await {
            if !self.prompts.coordinator.accepts_input(None, flow)
                && (!self.sessions.contains_key(&flow)
                    || !matches!(
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
                _ => self
                    .prompts
                    .coordinator
                    .fido_operations
                    .active_operation(flow),
            };
            let finished = matches!(&event, KeyOperationEvent::PromptDismissalRequested);
            let (state, session) = self.sessions.entry(flow).or_insert_with(|| {
                (
                    KeyOperationState::default(),
                    AuthenticationPromptWindows::new(),
                )
            });
            if let Some(command) = state.update(event) {
                self.prompts.handle(flow, session, operation, command);
            }
            if finished {
                self.sessions.remove(&flow);
            }
        }
        for (_, session) in self.sessions.into_values() {
            session.dismiss();
        }
    }
}
