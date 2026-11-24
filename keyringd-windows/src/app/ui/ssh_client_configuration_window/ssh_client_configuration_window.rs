use crate::app::ui::ssh_client_configuration_window::SshClientConfigurationCommandHandler;
use crate::app::ui::ssh_client_configuration_window::SshClientConfigurationNativeWindow;
use std::sync::{Arc, Mutex};

use crate::WindowsError;
use keyringd_ui_core::{
    ComponentModel,
    ssh_client_configuration_window::{SshClientConfigurationEvent, SshClientConfigurationState},
};

#[derive(Clone)]
pub(crate) struct SshClientConfigurationWindow {
    open_dialog: Arc<Mutex<()>>,
    #[cfg(feature = "test-support")]
    ssh_client_configuration: Option<(String, String)>,
}

impl SshClientConfigurationWindow {
    pub(crate) fn new(
        #[cfg(feature = "test-support")] ssh_client_configuration: Option<(String, String)>,
    ) -> Self {
        Self {
            open_dialog: Arc::new(Mutex::new(())),
            #[cfg(feature = "test-support")]
            ssh_client_configuration,
        }
    }

    pub(crate) fn open(&self, owner: usize) -> Result<(), WindowsError> {
        let Ok(_open_dialog) = self.open_dialog.try_lock() else {
            return Ok(());
        };
        let menu = SshClientConfigurationNativeWindow::new(owner);
        let handler = SshClientConfigurationCommandHandler::new(
            owner,
            #[cfg(feature = "test-support")]
            self.ssh_client_configuration.clone(),
        );
        let mut state = SshClientConfigurationState::new(
            handler
                .has_managed_agent_configuration()
                .map_err(|error| error.report_user_error(&error.user_message())),
        );
        state.update(SshClientConfigurationEvent::OpenRequested);
        loop {
            let mut requested = None;
            menu.show(&state, &mut |event| requested = Some(event))?;
            let Some(event) = requested else {
                return Ok(());
            };
            let mut command = state.update(event);
            while let Some(request) = command {
                let event = handler.handle(request, state.configured())?;
                let configured = matches!(
                    &event,
                    SshClientConfigurationEvent::ConfigurationFinished(Ok(()))
                );
                command = state.update(event);
                if configured {
                    handler.show_configured(state.configured())?;
                }
            }
        }
    }
}
