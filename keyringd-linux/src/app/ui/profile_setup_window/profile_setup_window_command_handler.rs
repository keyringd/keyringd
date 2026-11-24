use crate::app::ui::trust_entries_window::TrustEntriesWindow;
use crate::app::{AuthenticationPromptCommandHandler, LinuxApplication};
use gtk4::glib::object::Cast;
use gtk4::prelude::GtkWindowExt;
use keyringd_core::application::ProfileSetupRequest;
use keyringd_core::application::{ActionId, ApplicationError, PasskeyChoice, ProfileSetupFailure};
use keyringd_core::model::passkey::CredentialId;
use keyringd_ui_core::key_operation::KeyOperationCoordinator;
use keyringd_ui_core::profile_setup_window::{
    ProfileSetupSession, ProfileSetupWindowCommand, ProfileSetupWindowEvent,
};
use keyringd_ui_core::runtime::{UiEventSender, UiEventTarget};
use keyringd_ui_core::text as ui_text;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::Arc,
};

pub(super) struct ProfileSetupWindowCommandHandler {
    session: RefCell<Option<Arc<ProfileSetupSession>>>,
    dialog: libadwaita::Window,
    application: libadwaita::Application,
    profile_application: Arc<LinuxApplication>,
    prompts: Arc<KeyOperationCoordinator>,
    flow: Cell<Option<ActionId>>,
    key_flow: Cell<Option<ActionId>>,
    key_command_handler: RefCell<Option<AuthenticationPromptCommandHandler>>,
}
impl ProfileSetupWindowCommandHandler {
    pub(super) fn new(
        dialog: libadwaita::Window,
        application: libadwaita::Application,
        profile_application: Arc<LinuxApplication>,
        prompts: Arc<KeyOperationCoordinator>,
    ) -> Rc<Self> {
        Rc::new(Self {
            dialog,
            application,
            profile_application,
            prompts,
            session: RefCell::new(None),
            flow: Cell::new(None),
            key_flow: Cell::new(None),
            key_command_handler: RefCell::new(None),
        })
    }
    pub(super) fn log_span(&self) -> Option<tracing::Span> {
        self.flow
            .get()
            .map(|id| self.prompts.log_span(id, None, None))
    }
    pub(super) fn handle(
        self: &Rc<Self>,
        command: ProfileSetupWindowCommand,
        on_event: UiEventTarget<ProfileSetupWindowEvent>,
    ) {
        match command {
            ProfileSetupWindowCommand::Start(command) => {
                self.start_setup(command, on_event);
            }
            ProfileSetupWindowCommand::UseUsb(operation_id) => {
                let activated = self
                    .flow
                    .get()
                    .is_some_and(|id| self.prompts.fido_operations.activate_usb(id, operation_id));
                if !activated {
                    on_event.send(ProfileSetupWindowEvent::SecurityKeyActivationRejected);
                }
            }
            ProfileSetupWindowCommand::SelectCredential(result) => self.answer_selection(result),
            ProfileSetupWindowCommand::CancelOperation => {
                self.cancel();
            }
            ProfileSetupWindowCommand::CloseWindow => self.dialog.destroy(),
            ProfileSetupWindowCommand::OpenTrustEntries(id) => {
                TrustEntriesWindow::present(
                    &self.dialog.clone().upcast(),
                    Arc::clone(&self.profile_application),
                    id,
                );
            }
            ProfileSetupWindowCommand::RespondPin {
                request_id,
                command,
            } => self.answer_pin(request_id, command),
            ProfileSetupWindowCommand::Acknowledge(acknowledged) => {
                let _ = acknowledged.send(());
            }
            ProfileSetupWindowCommand::KeyOperation(command) => {
                if let Some(id) = self.key_flow.get()
                    && let Some(handler) = self.key_command_handler.borrow_mut().as_mut()
                {
                    handler.handle(command, id);
                }
            }
            ProfileSetupWindowCommand::OperationSettled { close_window: true } => {
                self.finish();
                self.dialog.destroy()
            }
            ProfileSetupWindowCommand::OperationSettled {
                close_window: false,
            } => self.finish(),
        }
    }
    fn begin(
        &self,
        choice: PasskeyChoice,
        cancellation: tokio_util::sync::CancellationToken,
    ) -> Arc<ProfileSetupSession> {
        let session = Arc::new(ProfileSetupSession::new(choice, cancellation));
        *self.session.borrow_mut() = Some(session.clone());
        session
    }

    fn answer_selection(&self, result: Result<CredentialId, ApplicationError>) {
        if let Some(session) = self.session.borrow().as_ref() {
            session.answer_selection(result);
        }
    }

    pub(super) fn cancel(&self) {
        if let Some(id) = self.flow.get() {
            self.prompts.cancel_action(id);
        }
        if let Some(session) = self.session.borrow().as_ref() {
            session.cancel();
        }
    }
    fn finish(&self) {
        if let Some(session) = self.session.borrow_mut().take() {
            session.cancel();
        }
    }
    fn answer_pin(
        &self,
        request_id: keyringd_fido_client::InteractionId,
        command: keyringd_ui_core::pin_prompt_window::PinPromptCommand,
    ) {
        if let Some(session) = self.session.borrow().as_ref() {
            session.answer_pin(request_id, command);
        }
    }

    fn start_setup(
        self: &Rc<Self>,
        request: keyringd_core::application::ProfileSetupRequest,
        on_event: UiEventTarget<ProfileSetupWindowEvent>,
    ) {
        let mut action = self.prompts.begin_profile_setup(request.passkey);
        let id = action.id();
        assert!(
            self.flow.replace(Some(id)).is_none(),
            "profile setup is already running"
        );
        let mut key_events = action.take_events();
        let cancellation = action.cancellation_token();
        let session = self.begin(request.passkey, cancellation.clone());
        // Progress, authentication and completion share one ordered UI stream.
        let (updates, mut receiver) = UiEventSender::channel();
        let authentication_updates = updates.clone();
        let (ui_events, mut ui_receiver) = UiEventSender::channel();
        self.key_flow.set(Some(id));
        self.key_command_handler
            .replace(Some(AuthenticationPromptCommandHandler::new(
                self.application.clone(),
                Some(self.dialog.clone().upcast()),
                Arc::clone(&self.prompts),
                Some(id),
                ui_events,
            )));
        let handler = Rc::clone(self);
        let authentication = session.clone();
        gtk4::glib::MainContext::ref_thread_default().spawn_local(async move {
            let _cancellation_guard = cancellation.drop_guard();
            loop {
                let (source,event) = tokio::select! {
                    event = key_events.receive() => { let Some(event) = event else { break }; event },
                    Some((source,event)) = ui_receiver.receive() => {
                        if !handler.prompts.accepts_input(Some(id), source) { event.reject(); continue; }
                        (source,event)
                    }
                };
                if source != id || handler.key_flow.get() != Some(id) { event.reject(); continue; }
                if let Some(event) = authentication.authentication_event(&handler.prompts, id, source, event) { authentication_updates.send(event); }
            }
            if handler.key_flow.get() == Some(id) {
                handler.key_flow.set(None);
                handler.key_command_handler.borrow_mut().take();
            }
        });
        let application = Arc::clone(&self.profile_application);
        tokio::spawn(async move {
            Self::execute(application, request, session, action, updates).await;
        });
        let handler = Rc::clone(self);
        gtk4::glib::MainContext::ref_thread_default().spawn_local(async move {
            loop {
                let event = receiver.receive().await.unwrap_or_else(|| {
                    ProfileSetupWindowEvent::Finished(Arc::new(Err(ProfileSetupFailure::Failed(
                        ApplicationError::InvariantViolation(
                            ui_text::PROFILE_SETUP_UNEXPECTED_STOP.into(),
                        ),
                    ))))
                });
                let finished = matches!(&event, ProfileSetupWindowEvent::Finished(_));
                if finished {
                    if handler.flow.get() != Some(id) {
                        break;
                    }
                    handler.flow.set(None);
                    handler.cancel();
                } else if handler.flow.get() != Some(id) {
                    continue;
                }
                on_event.send(event);
                if finished {
                    break;
                }
            }
        });
    }
    async fn execute(
        application: Arc<LinuxApplication>,
        request: ProfileSetupRequest,
        session: Arc<ProfileSetupSession>,
        action: keyringd_ui_core::key_operation::Action,
        events: keyringd_ui_core::runtime::UiEventSender<ProfileSetupWindowEvent>,
    ) {
        let action_id = action.id();
        let result = action
            .run(session.setup_profile(&application, &(), request, events.clone()))
            .await;
        action_id.span().in_scope(|| {
            if let Err(failure) = &result {
                ProfileSetupSession::log_failure(failure);
            }
            events.send(ProfileSetupWindowEvent::Finished(Arc::new(result)));
        });
    }
}
