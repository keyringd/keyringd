#[derive(Debug, thiserror::Error)]
#[error("XDG_RUNTIME_DIR is not set. Specify --socket explicitly.")]
pub(crate) struct StartupModeError;
