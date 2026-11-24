use super::SshClientConfigurationCopyTarget;
#[derive(Clone, Copy)]
pub enum SshClientConfigurationCommand {
    ConfirmConfiguration,
    Configure(bool),
    ConfirmCopy(SshClientConfigurationCopyTarget),
    Copy(SshClientConfigurationCopyTarget),
}
