use crate::app::ui::ssh_client_configuration_window::SshClientConfigurationEvent;

pub(super) struct SshClientConfigurationCallbackContext<'a> {
    pub(super) handler: &'a mut dyn FnMut(SshClientConfigurationEvent),
}
