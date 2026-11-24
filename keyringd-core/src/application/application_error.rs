use super::OperationRejectionReason;
use crate::service::DeviceKeyService;
use keyringd_fido_client::FidoClientError;

#[derive(Debug, thiserror::Error)]
pub enum ApplicationError {
    #[error("profile storage failed: {0}")]
    ProfileRepository(#[from] std::io::Error),
    #[error("profile changes are visible but could not be synchronized: {0}")]
    ProfileDurabilityUncertain(#[source] std::io::Error),
    #[error("invalid persisted profile state: {0}")]
    InvalidPersistedState(#[source] Box<dyn std::error::Error + Send + Sync>),
    #[error("operation rejected: {0}")]
    OperationRejected(#[from] OperationRejectionReason),
    #[error("{0} was not found")]
    NotFound(&'static str),
    #[error("operation conflict: {0}")]
    Conflict(&'static str),
    #[error("invalid authenticator response: {0}")]
    InvalidAuthenticatorResponse(#[source] Box<dyn std::error::Error + Send + Sync>),
    #[error("application invariant violated: {0}")]
    InvariantViolation(#[source] Box<dyn std::error::Error + Send + Sync>),
    #[error("device key operation failed: {0}")]
    DeviceKey(#[source] Box<dyn std::error::Error + Send + Sync>),
    #[error("hardware-backed Device Key storage is unavailable: {0}")]
    DeviceKeyUnavailable(#[source] Box<dyn std::error::Error + Send + Sync>),
    #[error("passkey operation failed: {0}")]
    Passkey(#[source] Box<dyn std::error::Error + Send + Sync>),
    #[error(
        "could not connect to the passkey tunnel service in time; check your network connection and try again"
    )]
    PasskeyHybridTunnelConnectionTimedOut(#[source] Box<dyn std::error::Error + Send + Sync>),
    #[error(
        "no compatible authenticator was found; connect a security key or use a passkey on your phone or tablet"
    )]
    PasskeyAuthenticatorNotFound(#[source] Box<dyn std::error::Error + Send + Sync>),
    #[error("the security key did not respond in time; try again and touch it when prompted")]
    PasskeySecurityKeyTimedOut(#[source] Box<dyn std::error::Error + Send + Sync>),
    #[error(
        "your phone or tablet did not connect in time; scan the QR code again and keep it near this PC"
    )]
    PasskeyHybridConnectionTimedOut(#[source] Box<dyn std::error::Error + Send + Sync>),
    #[error(
        "the connection to your phone or tablet was lost; scan the QR code again and complete the request on that device"
    )]
    PasskeyHybridConnectionLost(#[source] Box<dyn std::error::Error + Send + Sync>),
    #[error("the security-key PIN was incorrect; try again with the correct PIN")]
    PasskeyPinInvalid(#[source] Box<dyn std::error::Error + Send + Sync>),
    #[error(
        "the authenticator requires its PIN to be changed before it can be used; change the PIN with the authenticator's management tool and try again"
    )]
    PasskeyPinChangeRequired(#[source] Box<dyn std::error::Error + Send + Sync>),
    #[error(
        "the authenticator rejected the request; try again and approve it on the authenticator"
    )]
    PasskeyRequestRejected(#[source] Box<dyn std::error::Error + Send + Sync>),
    #[error("no passkey was found on the selected device")]
    PasskeyCredentialNotFound,
    #[error("operation canceled by the user")]
    OperationCanceled,
    #[error("certificate operation failed: {0}")]
    Certificate(#[source] Box<dyn std::error::Error + Send + Sync>),
}

impl From<FidoClientError> for ApplicationError {
    fn from(error: FidoClientError) -> Self {
        match error {
            FidoClientError::Cancelled => Self::OperationCanceled,
            error @ FidoClientError::HybridTunnelConnectionTimedOut(_) => {
                Self::PasskeyHybridTunnelConnectionTimedOut(Box::new(error))
            }
            error @ FidoClientError::TimedOut => Self::PasskeySecurityKeyTimedOut(Box::new(error)),
            error @ FidoClientError::NoAuthenticator => {
                Self::PasskeyAuthenticatorNotFound(Box::new(error))
            }
            error @ FidoClientError::PinChangeRequired(_) => {
                Self::PasskeyPinChangeRequired(Box::new(error))
            }
            FidoClientError::Ctap(keyringd_fido::CtapStatusCode::NoCredentials) => {
                Self::PasskeyCredentialNotFound
            }
            error @ FidoClientError::Ctap(keyringd_fido::CtapStatusCode::PinInvalid) => {
                Self::PasskeyPinInvalid(Box::new(error))
            }
            error @ FidoClientError::Ctap(keyringd_fido::CtapStatusCode::PinPolicyViolation) => {
                Self::PasskeyPinChangeRequired(Box::new(error))
            }
            error @ FidoClientError::Ctap(_) => Self::PasskeyRequestRejected(Box::new(error)),
            error => Self::Passkey(Box::new(error)),
        }
    }
}

impl ApplicationError {
    pub(crate) fn from_device_key_error<D: DeviceKeyService>(service: &D, error: D::Error) -> Self {
        if service.is_user_cancellation(&error) {
            Self::OperationCanceled
        } else {
            Self::DeviceKey(Box::new(error))
        }
    }

    fn log_level(&self) -> tracing::Level {
        match self {
            Self::OperationRejected(_)
            | Self::NotFound(_)
            | Self::Conflict(_)
            | Self::DeviceKeyUnavailable(_)
            | Self::PasskeyAuthenticatorNotFound(_)
            | Self::PasskeySecurityKeyTimedOut(_)
            | Self::PasskeyHybridConnectionTimedOut(_)
            | Self::PasskeyPinInvalid(_)
            | Self::PasskeyPinChangeRequired(_)
            | Self::PasskeyRequestRejected(_)
            | Self::PasskeyCredentialNotFound
            | Self::OperationCanceled => tracing::Level::INFO,
            Self::PasskeyHybridTunnelConnectionTimedOut(_)
            | Self::PasskeyHybridConnectionLost(_)
            | Self::InvalidAuthenticatorResponse(_) => tracing::Level::WARN,
            Self::ProfileRepository(_)
            | Self::ProfileDurabilityUncertain(_)
            | Self::InvalidPersistedState(_)
            | Self::InvariantViolation(_)
            | Self::DeviceKey(_)
            | Self::Passkey(_)
            | Self::Certificate(_) => tracing::Level::ERROR,
        }
    }

    pub fn log_failure(&self, message: &str) {
        use crate::util::ErrorChain;
        match self.log_level() {
            tracing::Level::INFO => {
                tracing::info!(error = ?self, error_chain = %ErrorChain::new(self), "{message}")
            }
            tracing::Level::WARN => {
                tracing::warn!(error = ?self, error_chain = %ErrorChain::new(self), "{message}")
            }
            _ => tracing::error!(error = ?self, error_chain = %ErrorChain::new(self), "{message}"),
        }
    }

    pub fn report_user_error(&self, message: &str) -> String {
        self.log_failure(message);
        message.to_owned()
    }

    /// Information for users. Keep internal error details in Display/Debug and Source.
    pub fn user_message(&self) -> String {
        match self {
            Self::ProfileRepository(_) => "Could not save the profile changes. Check that the profile storage folder is writable and has available space, then try again.".into(),
            Self::ProfileDurabilityUncertain(_) => "The profile changes are visible, but could not be synchronized to storage. They may be lost if the computer loses power.".into(),
            Self::InvalidPersistedState(_) => "Could not read the saved profiles. Restart the app. If the problem continues, restore your profiles from a backup.".into(),
            Self::OperationRejected(OperationRejectionReason::ProfileDisabled) => "This profile is disabled. Enable it before testing its SSH connection.".into(),
            Self::OperationRejected(OperationRejectionReason::SshIdentityUnavailable) => "The selected SSH identity is no longer available. Close this window and select an available identity.".into(),
            Self::OperationRejected(OperationRejectionReason::InvalidProfileName) => "Enter a profile name with 1 to 80 characters, without surrounding whitespace, line breaks, or control characters.".into(),
            Self::OperationRejected(_) => "The request could not be completed. Check the request and try again.".into(),
            Self::NotFound(_) => "The profile is no longer available. Return to the profile list and try again.".into(),
            Self::Conflict(_) => "Another profile operation is in progress. Wait for it to finish, then try again.".into(),
            Self::InvalidAuthenticatorResponse(_) | Self::InvariantViolation(_) | Self::Certificate(_) => "Could not complete the operation. Try again. If the problem continues, contact support.".into(),
            Self::DeviceKey(_) => "Could not use this PC's Device Key. Check the device unlock or key protection settings, then try again.".into(),
            Self::DeviceKeyUnavailable(_) => "Hardware-backed Device Key storage is unavailable on this computer. Check that TPM or Secure Enclave is supported, enabled, and accessible, then try again.".into(),
            Self::Passkey(_) => "Could not use the Passkey. Reconnect your security key or phone, then try again.".into(),
            Self::PasskeyHybridTunnelConnectionTimedOut(_)
            | Self::PasskeyAuthenticatorNotFound(_) | Self::PasskeySecurityKeyTimedOut(_)
            | Self::PasskeyHybridConnectionTimedOut(_) | Self::PasskeyHybridConnectionLost(_)
            | Self::PasskeyPinInvalid(_) | Self::PasskeyPinChangeRequired(_)
            | Self::PasskeyRequestRejected(_) | Self::PasskeyCredentialNotFound
            | Self::OperationCanceled => self.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    keyringd_testkit::scenario! {
        group "ApplicationError::log_level" {
            case "Classifies retries and cancellation as info connection loss as warn and invariant violations as error" {
                verify "each failure category determines its log level" {
                    for (error, expected) in [
                        (ApplicationError::PasskeySecurityKeyTimedOut("timeout".into()), tracing::Level::INFO),
                        (ApplicationError::PasskeyCredentialNotFound, tracing::Level::INFO),
                        (ApplicationError::PasskeyPinInvalid("invalid PIN".into()), tracing::Level::INFO),
                        (ApplicationError::OperationCanceled, tracing::Level::INFO),
                        (ApplicationError::PasskeyHybridConnectionLost("disconnected".into()), tracing::Level::WARN),
                        (ApplicationError::InvariantViolation("invalid state".into()), tracing::Level::ERROR),
                    ] {
                        assert_eq!(error.log_level(), expected, "{error:?}");
                    }
                }
            }
        }
        group "ApplicationError::user_message" {
            case "Omits internal diagnostic details from user messages" {
                verify "user messages do not include the diagnostic detail supplied with each error" {
                    let detail = "internal path /private/profile: os error 21";
                    let errors = [
                        ApplicationError::ProfileRepository(std::io::Error::other(detail)),
                        ApplicationError::InvalidPersistedState(detail.into()),
                        ApplicationError::InvalidAuthenticatorResponse(detail.into()),
                        ApplicationError::InvariantViolation(detail.into()),
                        ApplicationError::DeviceKey(Box::new(std::io::Error::other(detail))),
                        ApplicationError::Passkey(Box::new(std::io::Error::other(detail))),
                        ApplicationError::PasskeyPinInvalid(Box::new(std::io::Error::other(detail))),
                        ApplicationError::Certificate(detail.into()),
                    ];
                    for error in errors {
                        assert!(!error.user_message().contains(detail));
                    }
                }
            }
        }
        group "ApplicationError::from" {
            case "Classifies FIDO failures and preserves their causes except for cancellation and missing credentials" {
                verify "conversion selects the expected category and preserves the original cause when applicable" {
                    use keyringd_fido::CtapStatusCode;
                    use std::mem::discriminant;
                    let cause = || {
                        Box::new(std::io::Error::other("private diagnostic"))
                            as Box<dyn std::error::Error + Send + Sync>
                    };
                    let mappings = [
                        (
                            FidoClientError::Cancelled,
                            ApplicationError::OperationCanceled,
                            false,
                        ),
                        (
                            FidoClientError::TimedOut,
                            ApplicationError::PasskeySecurityKeyTimedOut(cause()),
                            true,
                        ),
                        (
                            FidoClientError::HybridTunnelConnectionTimedOut(cause()),
                            ApplicationError::PasskeyHybridTunnelConnectionTimedOut(cause()),
                            true,
                        ),
                        (
                            FidoClientError::NoAuthenticator,
                            ApplicationError::PasskeyAuthenticatorNotFound(cause()),
                            true,
                        ),
                        (
                            FidoClientError::Ctap(CtapStatusCode::NoCredentials),
                            ApplicationError::PasskeyCredentialNotFound,
                            false,
                        ),
                        (
                            FidoClientError::Ctap(CtapStatusCode::PinInvalid),
                            ApplicationError::PasskeyPinInvalid(cause()),
                            true,
                        ),
                        (
                            FidoClientError::Ctap(CtapStatusCode::PinPolicyViolation),
                            ApplicationError::PasskeyPinChangeRequired(cause()),
                            true,
                        ),
                        (
                            FidoClientError::pin_change_required(std::io::Error::other("private diagnostic")),
                            ApplicationError::PasskeyPinChangeRequired(cause()),
                            true,
                        ),
                        (
                            FidoClientError::Ctap(CtapStatusCode::InvalidCommand),
                            ApplicationError::PasskeyRequestRejected(cause()),
                            true,
                        ),
                        (
                            FidoClientError::OsCancelled,
                            ApplicationError::Passkey(cause()),
                            true,
                        ),
                        (
                            FidoClientError::Busy,
                            ApplicationError::Passkey(cause()),
                            true,
                        ),
                        (
                            FidoClientError::UnsupportedRequirement(cause()),
                            ApplicationError::Passkey(cause()),
                            true,
                        ),
                        (
                            FidoClientError::Interaction(cause()),
                            ApplicationError::Passkey(cause()),
                            true,
                        ),
                        (
                            FidoClientError::Transport(cause()),
                            ApplicationError::Passkey(cause()),
                            true,
                        ),
                        (
                            FidoClientError::Protocol(cause()),
                            ApplicationError::Passkey(cause()),
                            true,
                        ),
                        (
                            FidoClientError::Backend(cause()),
                            ApplicationError::Passkey(cause()),
                            true,
                        ),
                    ];
                    for (input, expected, retains_source) in mappings {
                        let input_kind = discriminant(&input);
                        let input_debug = format!("{input:?}");
                        let mapped = ApplicationError::from(input);
                        assert_eq!(
                            discriminant(&mapped),
                            discriminant(&expected),
                            "{input_debug}"
                        );
                        let source = std::error::Error::source(&mapped);
                        if retains_source {
                            let source = source.unwrap().downcast_ref::<FidoClientError>().unwrap();
                            assert_eq!(discriminant(source), input_kind, "{input_debug}");
                            assert_eq!(format!("{source:?}"), input_debug);
                        } else {
                            assert!(source.is_none(), "{input_debug}");
                        }
                    }
                }
            }
        }
    }
}
