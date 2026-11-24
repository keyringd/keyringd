use keyringd_ui_core::text as ui_text;
use std::sync::Arc;

use crate::app::WindowsApplication;

use crate::app::ui::util::AsyncMutation;

use windows_reactor::{HookRef, Updater};

use crate::app::ui::ComponentModel;

use crate::app::ui::AuxiliaryWindow;
use crate::app::ui::ssh_connection_test_window::ConnectionTestResultWindow;
use keyringd_core::model::ProfileId;
use keyringd_core::service::SshConnectionTestReport;
use keyringd_ui_core::ssh_connection_test_window::SshConnectionTestCommand;
use keyringd_ui_core::ssh_connection_test_window::SshConnectionTestEvent;
use keyringd_ui_core::ssh_connection_test_window::SshConnectionTestState;
use keyringd_ui_core::toast::ToastMessage;
use std::rc::Rc;

#[derive(Clone)]
pub(super) struct SshConnectionTestCommandHandler {
    profile_application: Arc<WindowsApplication>,
    profile_id: ProfileId,
    window: AuxiliaryWindow,
    mutation: AsyncMutation<SshConnectionTestReport>,
    task: HookRef<Option<tokio::task::AbortHandle>>,
    set_state: Updater<SshConnectionTestState>,
    #[cfg(feature = "test-support")]
    test_open_ssh: Option<Arc<crate::app::ui_test_support::ControlledOpenSsh>>,
}

impl SshConnectionTestCommandHandler {
    pub(super) fn new(
        profile_application: Arc<WindowsApplication>,
        profile_id: ProfileId,
        window: AuxiliaryWindow,
        mutation: AsyncMutation<SshConnectionTestReport>,
        task: HookRef<Option<tokio::task::AbortHandle>>,
        set_state: Updater<SshConnectionTestState>,
        #[cfg(feature = "test-support")] test_open_ssh: Option<
            Arc<crate::app::ui_test_support::ControlledOpenSsh>,
        >,
    ) -> Self {
        Self {
            profile_application,
            profile_id,
            window,
            mutation,
            task,
            set_state,
            #[cfg(feature = "test-support")]
            test_open_ssh,
        }
    }

    pub(super) fn handle(&self, command: SshConnectionTestCommand) -> Option<ToastMessage> {
        match command {
            SshConnectionTestCommand::Test { target, identity } => {
                let profile_application = Arc::clone(&self.profile_application);
                let id = self.profile_id.clone();
                #[cfg(feature = "test-support")]
                let test_open_ssh = self.test_open_ssh.clone();
                let task = self.mutation.start_cancellable(
                    keyringd_core::application::ActionId::generate().scope(async move {
                        #[cfg(feature = "test-support")]
                        let result = if let Some(service) = test_open_ssh {
                            profile_application
                                .test_ssh_connection(&id, identity, target, &*service)
                                .await
                        } else {
                            profile_application
                                .test_ssh_connection(
                                    &id,
                                    identity,
                                    target,
                                    &crate::system::WindowsOpenSshService,
                                )
                                .await
                        };
                        #[cfg(not(feature = "test-support"))]
                        let result = profile_application
                            .test_ssh_connection(
                                &id,
                                identity,
                                target,
                                &crate::system::WindowsOpenSshService,
                            )
                            .await;
                        result.map_err(|error| error.report_user_error(&error.user_message()))
                    }),
                );
                self.task.set(Some(task));
            }
            SshConnectionTestCommand::OpenResult { target, report } => {
                let set_state = self.set_state.clone();
                let handler = self.clone();
                let on_event = Rc::new(move |event| {
                    let (_, command) = crate::app::ui::reduce(&set_state, event);
                    command.and_then(|command| handler.handle(command))
                });
                let opened =
                    ConnectionTestResultWindow::open(target, report, self.window.hwnd(), on_event)
                        .map_err(|error| {
                            keyringd_core::util::report_user_error(
                                &error,
                                ui_text::OPEN_WINDOW_FAILED,
                            )
                        });
                self.set_state.call(move |mut state| {
                    state.update(SshConnectionTestEvent::ResultWindowOpened(opened));
                    state
                });
            }
            SshConnectionTestCommand::Close => self.window.close(),
            SshConnectionTestCommand::CopyTrustEntry(value) => {
                return Some(match crate::system::clipboard::copy(&value) {
                    Ok(()) => ToastMessage::Success(ui_text::SSH_CONNECTION_TEST_COPIED.into()),
                    Err(error) => ToastMessage::Error(keyringd_core::util::report_user_error(
                        &error,
                        keyringd_ui_core::text::COPY_FAILED,
                    )),
                });
            }
        }
        None
    }
}
