use super::AuthenticationPromptCommandHandler;
use crate::app::ui::{AppShell, MainThreadEventReceiverExt as _};
use dispatch2::MainThreadBound;
use keyringd_core::application::ActionId;
use keyringd_ui_core::ComponentModel;
use keyringd_ui_core::key_operation::{Action, KeyOperationCoordinator};
use keyringd_ui_core::key_operation::{KeyOperationCommand, KeyOperationEvent, KeyOperationState};
use keyringd_ui_core::runtime::UiEventSender;
use objc2::{MainThreadMarker, rc::Retained};
use std::{cell::RefCell, sync::Arc};
use tokio_util::sync::DropGuard;

/// Owns prompt state and cancellation for one window's authentication flow.
pub(crate) struct MacOsKeyOperationSession {
    id: ActionId,
    state: KeyOperationState,
    command_handler: AuthenticationPromptCommandHandler,
    _cancellation: DropGuard,
}

impl MacOsKeyOperationSession {
    pub(crate) fn start(
        runtime: &tokio::runtime::Handle,
        mtm: MainThreadMarker,
        coordinator: Arc<KeyOperationCoordinator>,
        flow: &mut Action,
        shell: Retained<AppShell>,
    ) -> Arc<MainThreadBound<RefCell<Option<Self>>>> {
        let id = flow.id();
        let events = flow.take_events();
        let cancellation = flow.cancellation_token();
        let (inputs, input_receiver) = UiEventSender::channel();
        let command_handler =
            AuthenticationPromptCommandHandler::new(mtm, coordinator, Some(id), shell, inputs);
        let session = Arc::new(MainThreadBound::new(
            RefCell::new(Some(Self {
                id,
                state: KeyOperationState::default(),
                command_handler,
                _cancellation: cancellation.drop_guard(),
            })),
            mtm,
        ));
        let weak = Arc::downgrade(&session);
        input_receiver.spawn_on_main(runtime, move |(id, event), mtm| {
            let Some(session) = weak.upgrade() else {
                event.reject();
                return;
            };
            let mut session = session.get(mtm).borrow_mut();
            if let Some(session) = session.as_mut() {
                session.dispatch(id, event, true);
            } else {
                event.reject();
            }
        });
        let backend = Arc::clone(&session);
        events.spawn_on_main(runtime, move |(id, event), mtm| {
            if let Some(session) = backend.get(mtm).borrow_mut().as_mut() {
                session.dispatch(id, event, false);
            } else {
                event.reject();
            }
        });
        session
    }

    pub(crate) fn dispatch(&mut self, id: ActionId, event: KeyOperationEvent, input: bool) {
        // Cleanup events must still settle after the operation stops accepting input.
        let terminal = matches!(
            &event,
            KeyOperationEvent::PromptDismissalRequested
                | KeyOperationEvent::HybridQrDismissed { .. }
        );
        if self.id != id || (!self.command_handler.accepts(id) && (input || !terminal)) {
            event.reject();
            return;
        }
        if let Some(command) = self.state.update(event) {
            self.command_handler.handle(command, id);
        }
    }
}

impl Drop for MacOsKeyOperationSession {
    fn drop(&mut self) {
        self.command_handler
            .handle(KeyOperationCommand::CloseOperationStatus, self.id);
    }
}
