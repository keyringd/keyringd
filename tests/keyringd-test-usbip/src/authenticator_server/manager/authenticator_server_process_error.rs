use std::process::ExitStatus;

#[derive(Debug, thiserror::Error)]
pub enum AuthenticatorServerProcessError {
    #[error("invalid AuthenticatorServer options: {0}")]
    InvalidOptions(String),
    #[error("AuthenticatorServer {operation}: {source}")]
    Io {
        operation: &'static str,
        #[source]
        source: std::io::Error,
    },
    #[error("AuthenticatorServer exited before readiness ({status}):\n{log_tail}")]
    ExitedEarly {
        status: ExitStatus,
        log_tail: String,
    },
    #[error("AuthenticatorServer startup timed out; {last_error}\n{log_tail}")]
    StartupTimedOut {
        last_error: String,
        log_tail: String,
    },
}
