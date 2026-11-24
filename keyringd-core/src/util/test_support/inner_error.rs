use super::RootError;

#[derive(Debug, thiserror::Error)]
#[error("inner failure")]
pub(in crate::util) struct InnerError {
    #[source]
    pub(in crate::util) source: RootError,
}
