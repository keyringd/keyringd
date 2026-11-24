#[derive(Debug, thiserror::Error)]
#[error("root cause")]
pub(in crate::util) struct RootError;
