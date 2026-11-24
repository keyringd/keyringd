use crate::WindowsError;
use crate::app::ui::ssh_client_configuration_window::SshClientConfigurationNativeWindow;
use keyringd_core::service::OpenSshService;
use keyringd_ui_core::ssh_client_configuration_window::{
    SshClientConfigurationCommand, SshClientConfigurationCopyTarget, SshClientConfigurationEvent,
};

pub(super) struct SshClientConfigurationCommandHandler {
    dialogs: SshClientConfigurationNativeWindow,
    #[cfg(feature = "test-support")]
    ssh_client_configuration: Option<(String, String)>,
}

impl SshClientConfigurationCommandHandler {
    pub(super) fn new(
        owner: usize,
        #[cfg(feature = "test-support")] ssh_client_configuration: Option<(String, String)>,
    ) -> Self {
        Self {
            dialogs: SshClientConfigurationNativeWindow::new(owner),
            #[cfg(feature = "test-support")]
            ssh_client_configuration,
        }
    }

    pub(super) fn handle(
        &self,
        command: SshClientConfigurationCommand,
        configured: bool,
    ) -> Result<SshClientConfigurationEvent, WindowsError> {
        match command {
            SshClientConfigurationCommand::ConfirmConfiguration => {
                Ok(if self.dialogs.confirm_windows_ssh_setup(configured)? {
                    SshClientConfigurationEvent::ConfirmationAccepted
                } else {
                    SshClientConfigurationEvent::ConfirmationCancelled
                })
            }
            SshClientConfigurationCommand::Configure(configured) => {
                Ok(SshClientConfigurationEvent::ConfigurationFinished(
                    self.configure_windows_ssh(configured)
                        .map_err(|error| error.report_user_error(&error.user_message())),
                ))
            }
            SshClientConfigurationCommand::ConfirmCopy(target) => {
                let confirmed = match target {
                    SshClientConfigurationCopyTarget::Wsl => {
                        self.dialogs.confirm_wsl_ssh_setting_copy()?
                    }
                    SshClientConfigurationCopyTarget::GitForWindows => {
                        self.dialogs.confirm_git_for_windows_ssh_setting_copy()?
                    }
                };
                Ok(if confirmed {
                    SshClientConfigurationEvent::ConfirmationAccepted
                } else {
                    SshClientConfigurationEvent::ConfirmationCancelled
                })
            }
            SshClientConfigurationCommand::Copy(target) => {
                let setting = match target {
                    SshClientConfigurationCopyTarget::Wsl => {
                        "export SSH_AUTH_SOCK=\"$XDG_RUNTIME_DIR/keyringd-agent.sock\""
                    }
                    SshClientConfigurationCopyTarget::GitForWindows => {
                        crate::system::windows_ssh_client_configuration::GIT_FOR_WINDOWS_SSH_SETTING
                    }
                };
                Ok(SshClientConfigurationEvent::CopyFinished(
                    crate::system::clipboard::copy(setting).map_err(|error| {
                        keyringd_core::util::report_user_error(
                            &error,
                            keyringd_ui_core::text::COPY_FAILED,
                        )
                    }),
                ))
            }
        }
    }

    pub(super) fn show_configured(&self, configured: bool) -> Result<(), WindowsError> {
        self.dialogs.show_windows_ssh_configured(!configured)?;
        Ok(())
    }

    pub(super) fn has_managed_agent_configuration(&self) -> Result<bool, WindowsError> {
        #[cfg(feature = "test-support")]
        if let Some((key, pipe)) = &self.ssh_client_configuration {
            return crate::system::windows_ssh_client_configuration::has_managed_agent_configuration_at(key, pipe)
                .map_err(|error| WindowsError::Environment(error.into()));
        }
        crate::system::WindowsOpenSshService
            .has_managed_agent_configuration()
            .map_err(|error| WindowsError::Environment(error.into()))
    }

    fn configure_windows_ssh(&self, configured: bool) -> Result<(), WindowsError> {
        #[cfg(feature = "test-support")]
        let result = if let Some((key, pipe)) = &self.ssh_client_configuration {
            if configured {
                crate::system::windows_ssh_client_configuration::configure_test(key, pipe)
            } else {
                crate::system::windows_ssh_client_configuration::unconfigure_test(key, pipe)
            }
        } else if configured {
            crate::system::WindowsOpenSshService.configure()
        } else {
            crate::system::WindowsOpenSshService.unconfigure()
        };
        #[cfg(not(feature = "test-support"))]
        let result = if configured {
            crate::system::WindowsOpenSshService.configure()
        } else {
            crate::system::WindowsOpenSshService.unconfigure()
        };
        result.map_err(|error| WindowsError::Environment(error.into()))
    }
}
