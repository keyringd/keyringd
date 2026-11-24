use super::ProfileSetupWindow;
use crate::app::MacOsApplication;
use dispatch2::MainThreadBound;
use keyringd_ui_core::key_operation::KeyOperationCoordinator;
use keyringd_ui_core::profile_setup_window::ProfileSetupSession;
use keyringd_ui_core::profile_setup_window::ProfileSetupWindowCommand;
use keyringd_ui_core::profile_setup_window::ProfileSetupWindowEvent;
use keyringd_ui_core::runtime::UiEventSender;
use objc2::MainThreadMarker;
use std::{
    cell::RefCell,
    sync::{Arc, Mutex},
};

#[derive(Clone)]
pub(super) struct ProfileSetupWindowCommandHandler {
    application: Arc<MacOsApplication>,
    runtime: tokio::runtime::Handle,
    interaction: Arc<KeyOperationCoordinator>,
    session: Arc<Mutex<Option<Arc<ProfileSetupSession>>>>,
}

impl ProfileSetupWindowCommandHandler {
    pub(super) fn new(
        application: Arc<MacOsApplication>,
        runtime: tokio::runtime::Handle,
        interaction: Arc<KeyOperationCoordinator>,
    ) -> Self {
        Self {
            application,
            runtime,
            interaction,
            session: Arc::new(Mutex::new(None)),
        }
    }

    pub(super) fn handle(
        &self,
        command: ProfileSetupWindowCommand,
        mtm: MainThreadMarker,
        setup: Arc<MainThreadBound<RefCell<ProfileSetupWindow>>>,
    ) {
        let application = self.application.clone();
        let runtime = self.runtime.clone();
        let interaction = self.interaction.clone();
        match command {
            ProfileSetupWindowCommand::Start(command) => {
                let mut flow = interaction.begin_profile_setup(command.passkey);
                let flow_id = flow.id();
                let updates = flow.take_events();
                let cancellation = flow.cancellation_token();
                let session = Arc::new(ProfileSetupSession::new(
                    command.passkey,
                    cancellation.clone(),
                ));
                *self.session.lock().unwrap() = Some(session.clone());
                setup
                    .get(mtm)
                    .borrow_mut()
                    .bind_flow(flow_id, cancellation.drop_guard());
                let (events, receiver) = UiEventSender::channel();
                let notifications = runtime.spawn(session.clone().receive_authentication(
                    interaction.clone(),
                    flow_id,
                    updates,
                    events.clone(),
                ));
                let delivered_setup = setup.clone();
                let delivered_handler = self.clone();
                use crate::app::ui::MainThreadEventReceiverExt as _;
                receiver.spawn_on_main(&runtime, move |event, mtm| {
                    if delivered_setup.get(mtm).borrow().flow_id() != Some(flow_id) {
                        return;
                    }
                    let command = match event {
                        ProfileSetupWindowEvent::Finished(result) => {
                            if let Some(session) = delivered_handler.session.lock().unwrap().take()
                            {
                                session.cancel();
                            }
                            delivered_setup
                                .get(mtm)
                                .borrow_mut()
                                .finish_flow(flow_id, result)
                        }
                        event => delivered_setup.get(mtm).borrow().dispatch(event),
                    };
                    if let Some(command) = command {
                        delivered_handler.handle(command, mtm, delivered_setup.clone());
                    }
                });
                runtime.clone().spawn(async move {
                    let result = flow
                        .run(session.setup_profile(&application, &(), command, events.clone()))
                        .await;

                    let _ = notifications.await;
                    let log_span = flow_id.span();
                    let _entered = log_span.enter();
                    if let Err(failure) = &result {
                        ProfileSetupSession::log_failure(failure);
                    }
                    flow_id.span().in_scope(|| {
                        events.send(ProfileSetupWindowEvent::Finished(Arc::new(result)))
                    });
                });
            }
            ProfileSetupWindowCommand::SelectCredential(result) => {
                if let Some(session) = self.session.lock().unwrap().as_ref() {
                    session.answer_selection(result);
                }
            }
            ProfileSetupWindowCommand::UseUsb(operation_id) => {
                let activated =
                    setup.get(mtm).borrow().flow_id().is_some_and(|id| {
                        interaction.fido_operations.activate_usb(id, operation_id)
                    });
                if !activated {
                    setup
                        .get(mtm)
                        .borrow()
                        .dispatch(ProfileSetupWindowEvent::SecurityKeyActivationRejected);
                }
            }
            ProfileSetupWindowCommand::CancelOperation => {
                if let Some(session) = self.session.lock().unwrap().as_ref() {
                    session.cancel();
                }
                if let Some(id) = setup.get(mtm).borrow().flow_id() {
                    interaction.cancel_action(id);
                }
            }
            ProfileSetupWindowCommand::CloseWindow => setup.get(mtm).borrow().hide(),
            ProfileSetupWindowCommand::OpenTrustEntries(id) => {
                setup.get(mtm).borrow().show_trust_entries(id)
            }
            ProfileSetupWindowCommand::RespondPin {
                request_id,
                command,
            } => {
                if let Some(session) = self.session.lock().unwrap().as_ref() {
                    session.answer_pin(request_id, command);
                }
            }
            ProfileSetupWindowCommand::Acknowledge(acknowledged) => {
                let _ = acknowledged.send(());
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
                    _ => tracing::error!("unexpected app-managed authentication prompt on macOS"),
                }
            }
            ProfileSetupWindowCommand::OperationSettled { close_window: true } => {
                setup.get(mtm).borrow().hide()
            }
            ProfileSetupWindowCommand::OperationSettled {
                close_window: false,
            } => {}
        }
    }
}
