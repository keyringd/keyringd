use super::SshClientConfigurationCopyTarget;
#[derive(Clone)]
pub enum SshClientConfigurationEvent {
    OpenRequested,
    ConfigurationChecked(Result<bool, String>),
    ConfigureRequested,
    CopyRequested(SshClientConfigurationCopyTarget),
    ConfirmationAccepted,
    ConfigurationFinished(Result<(), String>),
    CopyFinished(Result<(), String>),
    ConfirmationCancelled,
}
