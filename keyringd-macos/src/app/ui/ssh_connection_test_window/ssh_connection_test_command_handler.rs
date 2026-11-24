use super::ConnectionTestResultWindow;
use crate::app::MacOsApplication;
use crate::app::ui::toast::ToastOverlay;
use crate::system::open_ssh::MacOsOpenSshService;
use keyringd_core::model::ProfileId;
use keyringd_ui_core::runtime::UiEventSender;
use keyringd_ui_core::ssh_connection_test_window::{
    SshConnectionTestCommand, SshConnectionTestEvent,
};
use keyringd_ui_core::{text as ui_text, toast::ToastMessage};
use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2_app_kit::NSWindow;
use objc2_foundation::NSString;
use std::sync::Arc;

pub(super) struct SshConnectionTestCommandHandler {
    open_ssh: Arc<MacOsOpenSshService>,
    application: Arc<MacOsApplication>,
    profile_id: ProfileId,
    runtime: tokio::runtime::Handle,
    events: UiEventSender<SshConnectionTestEvent>,
    parent: Retained<NSWindow>,
    sheet: Retained<NSWindow>,
    result_toast: Option<ToastOverlay>,
}

impl SshConnectionTestCommandHandler {
    pub(super) fn new(
        open_ssh: Arc<MacOsOpenSshService>,
        application: Arc<MacOsApplication>,
        profile_id: ProfileId,
        runtime: tokio::runtime::Handle,
        events: UiEventSender<SshConnectionTestEvent>,
        parent: Retained<NSWindow>,
        sheet: Retained<NSWindow>,
    ) -> Self {
        Self {
            open_ssh,
            application,
            profile_id,
            runtime,
            events,
            parent,
            sheet,
            result_toast: None,
        }
    }

    pub(super) fn handle(&mut self, command: SshConnectionTestCommand, mtm: MainThreadMarker) {
        match command {
            SshConnectionTestCommand::Test { target, identity } => {
                let application = Arc::clone(&self.application);
                let open_ssh = Arc::clone(&self.open_ssh);
                let profile_id = self.profile_id.clone();
                let events = self.events.clone();
                self.runtime
                    .spawn(
                        keyringd_core::application::ActionId::generate().scope(async move {
                            let result = application
                                .test_ssh_connection(&profile_id, identity, target, &*open_ssh)
                                .await
                                .map_err(|error| error.report_user_error(&error.user_message()));
                            events.send(SshConnectionTestEvent::TestFinished(result));
                        }),
                    );
            }
            SshConnectionTestCommand::OpenResult { target, report } => {
                self.result_toast = Some(ConnectionTestResultWindow::present(
                    &self.sheet,
                    &target,
                    &report,
                    self.events.clone(),
                    mtm,
                ));
                self.events
                    .send(SshConnectionTestEvent::ResultWindowOpened(Ok(())));
            }
            SshConnectionTestCommand::Close => {
                if self.sheet.sheetParent().is_some() {
                    self.parent.endSheet(&self.sheet);
                }
                self.result_toast = None;
            }
            SshConnectionTestCommand::CopyTrustEntry(value) => {
                let pasteboard = crate::system::clipboard::pasteboard(mtm);
                pasteboard.clearContents();
                let copied = pasteboard.setString_forType(&NSString::from_str(&value), unsafe {
                    objc2_app_kit::NSPasteboardTypeString
                });
                if let Some(toast) = &self.result_toast {
                    toast.show(
                        if copied {
                            ToastMessage::Success(ui_text::SSH_CONNECTION_TEST_COPIED.into())
                        } else {
                            ToastMessage::Error(ui_text::COPY_FAILED.into())
                        },
                        mtm,
                    );
                }
            }
        }
    }
}
