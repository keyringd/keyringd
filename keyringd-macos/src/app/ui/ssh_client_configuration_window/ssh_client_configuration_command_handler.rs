use super::{SshClientConfigurationCommand, SshClientConfigurationEvent};
use crate::app::SshAgentSelectionService;
use block2::RcBlock;
use keyringd_ui_core::{runtime::UiEventSender, text as ui_text};
use objc2::{MainThreadMarker, rc::Retained};
use objc2_app_kit::{NSAccessibility, NSAlert, NSAlertFirstButtonReturn, NSWindow};
use objc2_foundation::NSString;
use std::sync::Arc;

pub(super) struct SshClientConfigurationCommandHandler {
    selection: Arc<SshAgentSelectionService>,
    runtime: tokio::runtime::Handle,
    events: UiEventSender<SshClientConfigurationEvent>,
    window: Retained<NSWindow>,
}
impl SshClientConfigurationCommandHandler {
    pub(super) fn new(
        selection: Arc<SshAgentSelectionService>,
        runtime: tokio::runtime::Handle,
        events: UiEventSender<SshClientConfigurationEvent>,
        window: Retained<NSWindow>,
    ) -> Self {
        Self {
            selection,
            runtime,
            events,
            window,
        }
    }
    pub(super) fn has_managed_agent_configuration(&self) -> Result<bool, String> {
        self.selection
            .has_managed_agent_configuration()
            .map_err(|error| {
                keyringd_core::util::report_user_error(
                    &error,
                    ui_text::SSH_CLIENT_CONFIGURATION_FAILED,
                )
            })
    }
    pub(super) fn handle(
        &self,
        command: SshClientConfigurationCommand,
        configured: bool,
        mtm: MainThreadMarker,
    ) {
        match command {
            SshClientConfigurationCommand::ConfirmConfiguration => {
                self.confirm_configuration(configured, mtm)
            }
            SshClientConfigurationCommand::Configure(configured) => {
                let selection = Arc::clone(&self.selection);
                let events = self.events.clone();
                self.runtime.spawn(async move {
                    events.send(Self::configure(selection, configured).await);
                });
            }
            SshClientConfigurationCommand::ConfirmCopy(_)
            | SshClientConfigurationCommand::Copy(_) => {
                unreachable!("macOS SSH settings only offer native configuration")
            }
        }
    }
    fn confirm_configuration(&self, configured: bool, mtm: MainThreadMarker) {
        self.present_confirmation(
            keyringd_ui_core::ssh_client_configuration_window::confirmation_heading(configured),
            if configured { "Restore the macOS SSH agent for new connections, including connections from apps that are already open. Keyringd will keep running." } else { "Use Keyringd for new SSH connections, including connections from apps that are already open. This choice will also be applied at future logins." },
            if configured { ui_text::SSH_CLIENT_CONFIGURATION_UNCONFIGURE } else { ui_text::SSH_CLIENT_CONFIGURATION_CONFIGURE },
            SshClientConfigurationEvent::ConfirmationAccepted,
            mtm,
        );
    }

    fn present_confirmation(
        &self,
        title: &str,
        detail: &str,
        action: &str,
        confirmed: SshClientConfigurationEvent,
        mtm: MainThreadMarker,
    ) {
        let alert = NSAlert::new(mtm);
        alert.setMessageText(&NSString::from_str(title));
        alert.setInformativeText(&NSString::from_str(detail));
        alert
            .window()
            .setAccessibilityIdentifier(Some(&NSString::from_str(
                "getting-started.confirmation.prompt",
            )));
        alert
            .addButtonWithTitle(&NSString::from_str(action))
            .setAccessibilityIdentifier(Some(&NSString::from_str(
                "getting-started.confirmation.confirm",
            )));
        alert
            .addButtonWithTitle(&NSString::from_str(ui_text::CANCEL))
            .setAccessibilityIdentifier(Some(&NSString::from_str(
                "getting-started.confirmation.cancel",
            )));
        let events = self.events.clone();
        let completion = RcBlock::new(move |response| {
            let event = if response == NSAlertFirstButtonReturn {
                confirmed.clone()
            } else {
                SshClientConfigurationEvent::ConfirmationCancelled
            };
            events.send(event);
        });
        alert.beginSheetModalForWindow_completionHandler(&self.window, Some(&completion));
    }
    async fn configure(
        service: Arc<SshAgentSelectionService>,
        configured: bool,
    ) -> SshClientConfigurationEvent {
        let result = tokio::task::spawn_blocking(move || {
            if configured {
                service.configure()
            } else {
                service.unconfigure()
            }
            .map_err(|error| {
                keyringd_core::util::report_user_error(
                    &error,
                    ui_text::SSH_CLIENT_CONFIGURATION_FAILED,
                )
            })
        })
        .await
        .map_err(|error| {
            keyringd_core::util::report_user_error(&error, ui_text::SSH_CLIENT_CONFIGURATION_FAILED)
        })
        .and_then(|result| result);
        SshClientConfigurationEvent::ConfigurationFinished(result)
    }
}
