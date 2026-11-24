use super::ConnectionTestResultWindow;
use crate::app::LinuxApplication;
use crate::system::open_ssh::LinuxOpenSshService;
use gtk4::gdk::prelude::DisplayExt;
use gtk4::glib::{WeakRef, object::Cast};
use gtk4::prelude::GtkWindowExt;
use keyringd_core::model::ProfileId;
use keyringd_ui_core::runtime::UiEventSender;
use keyringd_ui_core::ssh_connection_test_window::{
    SshConnectionTestCommand, SshConnectionTestEvent,
};
use keyringd_ui_core::{text as ui_text, toast::ToastMessage};
use std::{rc::Rc, sync::Arc};

#[derive(Clone)]
pub(super) struct SshConnectionTestCommandHandler {
    open_ssh: Arc<LinuxOpenSshService>,
    application: Arc<LinuxApplication>,
    profile_id: ProfileId,
    dialog: WeakRef<libadwaita::MessageDialog>,
}

impl SshConnectionTestCommandHandler {
    pub(super) fn new(
        open_ssh: Arc<LinuxOpenSshService>,
        application: Arc<LinuxApplication>,
        profile_id: ProfileId,
        dialog: WeakRef<libadwaita::MessageDialog>,
    ) -> Self {
        Self {
            open_ssh,
            application,
            profile_id,
            dialog,
        }
    }

    pub(super) fn handle(
        &self,
        command: SshConnectionTestCommand,
        on_event: Rc<dyn Fn(SshConnectionTestEvent) -> Option<ToastMessage>>,
    ) -> Option<ToastMessage> {
        match command {
            SshConnectionTestCommand::Test { target, identity } => {
                let application = Arc::clone(&self.application);
                let open_ssh = Arc::clone(&self.open_ssh);
                let profile_id = self.profile_id.clone();
                let (finished, mut receiver) = UiEventSender::channel();
                tokio::spawn(
                    keyringd_core::application::ActionId::generate().scope(async move {
                        let result = application
                            .test_ssh_connection(&profile_id, identity, target, &*open_ssh)
                            .await
                            .map_err(|error| error.report_user_error(&error.user_message()));
                        finished.send(SshConnectionTestEvent::TestFinished(result));
                    }),
                );
                let dialog = self.dialog.clone();
                gtk4::glib::MainContext::ref_thread_default().spawn_local(async move {
                    if let Some(event) = receiver.receive().await
                        && dialog.upgrade().is_some()
                    {
                        on_event(event);
                    }
                });
            }
            SshConnectionTestCommand::OpenResult { target, report } => {
                let dialog = self.dialog.upgrade()?;
                ConnectionTestResultWindow::present(
                    dialog.upcast_ref(),
                    &target,
                    &report,
                    Rc::clone(&on_event),
                );
                on_event(SshConnectionTestEvent::ResultWindowOpened(Ok(())));
            }
            SshConnectionTestCommand::Close => {
                if let Some(dialog) = self.dialog.upgrade() {
                    dialog.close();
                }
            }
            SshConnectionTestCommand::CopyTrustEntry(value) => {
                return Some(if let Some(display) = gtk4::gdk::Display::default() {
                    display.clipboard().set_text(&value);
                    ToastMessage::Success(ui_text::SSH_CONNECTION_TEST_COPIED.into())
                } else {
                    ToastMessage::Error(ui_text::COPY_FAILED.into())
                });
            }
        }
        None
    }
}
