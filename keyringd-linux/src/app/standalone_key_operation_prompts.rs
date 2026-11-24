use keyringd_core::application::ActionId;

use crate::app::AuthenticationPromptCommandHandler;
use crate::app::ui::{ComponentRuntime, UiEventReceiver};
use keyringd_ui_core::key_operation::KeyOperationState;
use keyringd_ui_core::key_operation::{KeyOperationCoordinator, KeyOperationEvent};
use std::sync::Arc;

/// Desktop-owned prompts for requests that have no parent window.
pub(crate) struct StandaloneKeyOperationPrompts {
    runtime: ComponentRuntime<KeyOperationState>,
    command_handler: AuthenticationPromptCommandHandler,
}

impl StandaloneKeyOperationPrompts {
    pub(crate) fn start(
        application: &libadwaita::Application,
        interaction: Arc<KeyOperationCoordinator>,
        mut events: UiEventReceiver<(ActionId, KeyOperationEvent)>,
    ) {
        let application = application.clone();
        gtk4::glib::MainContext::ref_thread_default().spawn_local(async move {
            let (ui_events, mut ui_receiver) = keyringd_ui_core::runtime::UiEventSender::channel();
            let mut prompts = Self { runtime: ComponentRuntime::new(KeyOperationState::default()), command_handler: AuthenticationPromptCommandHandler::new(application, None, interaction, None, ui_events) };
            let mut displayed_flow = None;
            loop {
                let (source, event) = tokio::select! {
                    event = events.receive() => {
                        let Some((source, event)) = event else { break };
                        let terminal = matches!(&event, KeyOperationEvent::HybridQrDismissed { .. } | KeyOperationEvent::PromptDismissalRequested);
                        if !prompts.command_handler.accepts(source) && !(displayed_flow == Some(source) && terminal) {
                            event.reject();
                            continue;
                        }
                        if displayed_flow != Some(source) {
                            prompts.command_handler.reset();
                            prompts.runtime.reset(KeyOperationState::default(), |_| {});
                            displayed_flow = Some(source);
                        }
                        (source, event)
                    }
                    Some((source,event)) = ui_receiver.receive() => {
                        Self::dispatch_ui_event(&prompts.runtime, &mut prompts.command_handler, source, event);
                        continue;
                    }
                };
                if let Some(command) = prompts.runtime.dispatch(event) { prompts.command_handler.handle(command, source); }
            }
        });
    }

    fn dispatch_ui_event(
        runtime: &ComponentRuntime<KeyOperationState>,
        command_handler: &mut AuthenticationPromptCommandHandler,
        source: ActionId,
        event: KeyOperationEvent,
    ) {
        if !command_handler.accepts(source) {
            event.reject();
            return;
        }
        if let Some(command) = runtime.dispatch(event) {
            command_handler.handle(command, source);
        }
    }
}

#[cfg(test)]
mod tests {
    use keyringd_fido_client::OperationId;
    use keyringd_ui_core::key_operation::{KeyOperationCoordinator, KeyOperationEvent};

    keyringd_testkit::scenario! {
        group "StandaloneKeyOperationPrompts::dispatch_ui_event" {
            case "Expired prompt commands cannot cancel or activate the next action" {
                verify "ignore cancel and activation messages from the previous action" {
                    gtk4::test_synced(|| {
                        let tokio_runtime = tokio::runtime::Runtime::new().unwrap();
                        tokio_runtime.block_on(async {
                            let (interaction, _) = KeyOperationCoordinator::channel();
                            let (sender, _) = keyringd_ui_core::runtime::UiEventSender::channel();
                            let first_action = interaction.actions.begin_standalone(sender).await;
                            let first = first_action.id();
                            drop(first_action);
                            let (sender, _) = keyringd_ui_core::runtime::UiEventSender::channel();
                            let _second_action = interaction.actions.begin_standalone(sender).await;
                            let second = _second_action.id();
                            let cancellation = interaction.actions.cancellation(second).unwrap();
                            let (activation, activated) = tokio::sync::watch::channel(false);
                            let operation = OperationId::new(123);
                            interaction
                                .fido_operations
                                .register(operation, second);
                            interaction
                                .fido_operations
                                .set_usb_activation(operation, activation);
                            let (events, _receiver) = keyringd_ui_core::runtime::UiEventSender::channel();
                            let mut command_handler = crate::app::AuthenticationPromptCommandHandler::new(
                                libadwaita::Application::builder().build(),
                                None,
                                interaction.clone(),
                                None,
                                events,
                            );
                            let model = crate::app::ui::ComponentRuntime::new(
                                keyringd_ui_core::key_operation::KeyOperationState::default(),
                            );
                            let (presented, _) = tokio::sync::oneshot::channel();
                            model.dispatch(KeyOperationEvent::HybridQrRequested {
                                prompt_id: keyringd_fido_client::InteractionId::new(operation, 1),
                                value: "FIDO:/test".into(),
                                context: Some(keyringd_ui_core::key_operation::AuthenticatorPromptPurpose::AgentRequest),
                                security_key_available: true,
                                presented,
                            });
                            for event in [
                                KeyOperationEvent::CancelRequested,
                                KeyOperationEvent::UseSecurityKeyRequested,
                            ] {
                                super::StandaloneKeyOperationPrompts::dispatch_ui_event(
                                    &model,
                                    &mut command_handler,
                                    first,
                                    event,
                                );
                            }
                            assert!(!cancellation.is_cancelled());
                            assert!(!*activated.borrow());
                            super::StandaloneKeyOperationPrompts::dispatch_ui_event(
                                &model,
                                &mut command_handler,
                                second,
                                KeyOperationEvent::UseSecurityKeyRequested,
                            );
                            assert!(*activated.borrow());
                            super::StandaloneKeyOperationPrompts::dispatch_ui_event(
                                &model,
                                &mut command_handler,
                                second,
                                KeyOperationEvent::CancelRequested,
                            );
                            assert!(cancellation.is_cancelled());
                        });
                    });
                }
            }
        }
    }
}
