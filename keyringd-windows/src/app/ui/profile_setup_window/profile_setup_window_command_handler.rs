use crate::app::WindowsApplication;
use crate::system::device_key::CngOperationContext;
use keyringd_core::application::ActionId;

use crate::app::ui::trust_entries_window::TrustEntriesWindow;

use windows_reactor::{HookRef, RenderCx};

use keyringd_ui_core::profile_setup_window::ProfileSetupSession;
use std::cell::Cell;
use std::rc::Rc;
use std::sync::Arc;

use keyringd_ui_core::runtime::UiEventSender;

use keyringd_ui_core::profile_setup_window::ProfileSetupWindowCommand;
use keyringd_ui_core::profile_setup_window::ProfileSetupWindowEvent;

#[derive(Clone)]
pub(super) struct ProfileSetupWindowCommandHandler {
    session: HookRef<Option<Arc<ProfileSetupSession>>>,
    coordinator: Arc<keyringd_ui_core::key_operation::KeyOperationCoordinator>,
    application: Arc<WindowsApplication>,
    flow: HookRef<Option<ActionId>>,
    events: UiEventSender<ProfileSetupWindowEvent>,
    owner: Rc<Cell<usize>>,
    runtime: tokio::runtime::Handle,
}

impl ProfileSetupWindowCommandHandler {
    pub(super) fn use_in(
        cx: &mut RenderCx,
        application: Arc<WindowsApplication>,
        runtime: tokio::runtime::Handle,
        owner: Rc<Cell<usize>>,
        events: UiEventSender<ProfileSetupWindowEvent>,
        coordinator: Arc<keyringd_ui_core::key_operation::KeyOperationCoordinator>,
    ) -> Self {
        let handler = Self {
            session: cx.use_ref(None::<Arc<ProfileSetupSession>>),
            flow: cx.use_ref(None),
            coordinator,
            application,
            runtime,
            owner,
            events,
        };
        cx.use_effect_with_cleanup((), {
            let handler = handler.clone();
            move || Some(move || handler.cancel())
        });
        handler
    }

    fn cancel(&self) {
        if let Some(session) = self.session.get_cloned() {
            session.cancel();
        }
        if let Some(flow) = self.flow.get_cloned() {
            self.coordinator.cancel_action(flow);
        }
    }

    pub(super) fn handle(
        &self,
        command: ProfileSetupWindowCommand,
    ) -> Option<ProfileSetupWindowEvent> {
        match command {
            ProfileSetupWindowCommand::Start(request) => {
                if self.flow.get_cloned().is_some() {
                    return None;
                }
                let mut flow = self.coordinator.begin_profile_setup(request.passkey);
                let flow_id = flow.id();
                let updates = flow.take_events();
                let cancellation = flow.cancellation_token();
                let session = Arc::new(ProfileSetupSession::new(request.passkey, cancellation));
                self.session.set(Some(session.clone()));
                self.flow.set(Some(flow_id));
                let notifications = self.runtime.spawn(session.clone().receive_authentication(
                    self.coordinator.clone(),
                    flow_id,
                    updates,
                    self.events.clone(),
                ));
                let application = self.application.clone();
                let owner = self.owner.get();
                let events = self.events.clone();
                self.runtime.spawn(async move {
                    let device_key_context = CngOperationContext::for_window(owner);
                    let result = flow
                        .run(session.setup_profile(
                            &application,
                            &device_key_context,
                            request,
                            events.clone(),
                        ))
                        .await;

                    let _ = notifications.await;
                    flow_id.span().in_scope(|| {
                        if let Err(failure) = &result {
                            ProfileSetupSession::log_failure(failure);
                        }
                        events.send(ProfileSetupWindowEvent::Finished(Arc::new(result)));
                    });
                });
                None
            }
            ProfileSetupWindowCommand::SelectCredential(result) => {
                if let Some(session) = self.session.get_cloned() {
                    session.answer_selection(result);
                }
                None
            }
            ProfileSetupWindowCommand::CancelOperation => {
                self.cancel();
                None
            }
            ProfileSetupWindowCommand::CloseWindow => {
                super::ProfileSetupWindow::close(self.owner.get());
                None
            }
            ProfileSetupWindowCommand::OpenTrustEntries(id) => {
                Some(ProfileSetupWindowEvent::TrustEntriesOpened(
                    TrustEntriesWindow::open_modal(
                        self.application.clone(),
                        self.runtime.clone(),
                        id,
                        self.owner.get(),
                    )
                    .map_err(|error| {
                        keyringd_core::util::report_user_error(
                            &error,
                            keyringd_ui_core::text::OPEN_PUBLIC_KEYS_FAILED,
                        )
                    }),
                ))
            }
            ProfileSetupWindowCommand::UseUsb(operation_id) => {
                let activated = self.flow.get_cloned().is_some_and(|flow| {
                    self.coordinator
                        .fido_operations
                        .activate_usb(flow, operation_id)
                });
                (!activated).then(|| keyringd_ui_core::profile_setup_window::ProfileSetupWindowEvent::SecurityKeyActivationRejected)
            }
            ProfileSetupWindowCommand::RespondPin {
                request_id,
                command,
            } => {
                if let Some(session) = self.session.get_cloned() {
                    session.answer_pin(request_id, command);
                }
                None
            }
            ProfileSetupWindowCommand::Acknowledge(acknowledged) => {
                let _ = acknowledged.send(());
                None
            }
            ProfileSetupWindowCommand::KeyOperation(command) => {
                use keyringd_ui_core::key_operation::KeyOperationCommand;
                match command {
                    KeyOperationCommand::CloseOperationStatus => {}
                    KeyOperationCommand::AnswerPin { value, response }
                    | KeyOperationCommand::AnswerDeviceKeyPassword { value, response } => {
                        let _ = response.send(value);
                    }
                    KeyOperationCommand::AnswerDeviceKeyPasswordChange { value, response } => {
                        let _ = response.send(value);
                    }
                    _ => tracing::error!("unexpected app-managed authentication prompt on Windows"),
                }
                None
            }
            ProfileSetupWindowCommand::OperationSettled { close_window } => {
                self.cancel();
                self.flow.set(None);
                self.session.set(None);
                if close_window {
                    super::ProfileSetupWindow::close(self.owner.get());
                }
                None
            }
        }
    }
}
