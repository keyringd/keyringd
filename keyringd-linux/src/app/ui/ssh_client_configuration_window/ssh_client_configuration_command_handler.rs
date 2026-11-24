use crate::system::open_ssh::LinuxOpenSshService;
use gtk4::glib::WeakRef;
use gtk4::prelude::GtkWindowExt;
use keyringd_core::service::OpenSshService;
use keyringd_ui_core::runtime::UiEventSender;
use keyringd_ui_core::ssh_client_configuration_window::{
    SshClientConfigurationCommand, SshClientConfigurationEvent,
};
use keyringd_ui_core::text as ui_text;
use libadwaita::prelude::MessageDialogExt;
use std::{rc::Rc, sync::Arc};

#[derive(Clone)]
pub(super) struct SshClientConfigurationCommandHandler {
    open_ssh: Arc<LinuxOpenSshService>,
    parent: WeakRef<gtk4::Window>,
}
impl SshClientConfigurationCommandHandler {
    pub(super) fn new(open_ssh: Arc<LinuxOpenSshService>, parent: WeakRef<gtk4::Window>) -> Self {
        Self { open_ssh, parent }
    }
    pub(super) fn has_managed_agent_configuration(&self) -> Result<bool, String> {
        self.open_ssh.has_managed_agent_configuration().map_err(|error| keyringd_core::util::report_user_error(&error, "Could not check SSH_AUTH_SOCK configuration. Check that your environment configuration folder is readable, then try again."))
    }
    pub(super) fn handle(
        &self,
        command: SshClientConfigurationCommand,
        configured: bool,
        on_event: Rc<dyn Fn(SshClientConfigurationEvent)>,
    ) {
        match command {
            SshClientConfigurationCommand::ConfirmConfiguration => {
                let Some(parent) = self.parent.upgrade() else {
                    return;
                };
                let removing = configured;
                let action = if removing {
                    ui_text::SSH_CLIENT_CONFIGURATION_UNCONFIGURE
                } else {
                    ui_text::SSH_CLIENT_CONFIGURATION_CONFIGURE
                };
                let confirmation = crate::app::ui::message_dialog_builder()
                .transient_for(&parent)
                .heading(keyringd_ui_core::ssh_client_configuration_window::confirmation_heading(removing))
                .body(if removing { "This removes Keyringd's SSH_AUTH_SOCK setting and restores the default behavior of the GCR and OpenSSH agent sockets. Sign out and back in to apply the change." } else { "This sets SSH_AUTH_SOCK for your user and prevents the GCR and OpenSSH agent sockets from changing it. Sign out and back in to apply the change." })
                .close_response("cancel")
                .default_response("configure")
                .build();
                confirmation.add_response("cancel", ui_text::CANCEL);
                confirmation.add_response("configure", action);
                confirmation.set_response_appearance(
                    "configure",
                    libadwaita::ResponseAppearance::Suggested,
                );
                confirmation.connect_response(None, move |confirmation, response| {
                    confirmation.close();
                    on_event(if response == "configure" {
                        SshClientConfigurationEvent::ConfirmationAccepted
                    } else {
                        SshClientConfigurationEvent::ConfirmationCancelled
                    });
                });
                confirmation.present();
            }
            SshClientConfigurationCommand::Configure(configured) => {
                let service = Arc::clone(&self.open_ssh);
                let (finished, mut receiver) = UiEventSender::channel();
                tokio::spawn(async move {
                    finished.send(Self::configure(service, configured).await);
                });
                gtk4::glib::MainContext::ref_thread_default().spawn_local(async move {
                    if let Some(event) = receiver.receive().await {
                        on_event(event);
                    }
                });
            }
            SshClientConfigurationCommand::ConfirmCopy(_)
            | SshClientConfigurationCommand::Copy(_) => {
                unreachable!("Linux SSH settings only offer native configuration")
            }
        }
    }
    async fn configure(
        open_ssh: Arc<LinuxOpenSshService>,
        configured: bool,
    ) -> SshClientConfigurationEvent {
        let result = tokio::task::spawn_blocking(move || {
            if configured {
                open_ssh.configure().map_err(|error| keyringd_core::util::report_user_error(&error, "Could not configure SSH_AUTH_SOCK. Check that your environment configuration folder is writable, then try again."))
            } else {
                open_ssh.unconfigure().map_err(|error| keyringd_core::util::report_user_error(&error, "Could not remove SSH_AUTH_SOCK configuration. Check that your environment configuration folder is writable, then try again."))
            }
        })
        .await
        .map_err(|error| keyringd_core::util::report_user_error(&error, ui_text::SSH_CLIENT_CONFIGURATION_FAILED))
        .and_then(|result| result);
        SshClientConfigurationEvent::ConfigurationFinished(result)
    }
}
