use super::InnerError;

#[derive(Debug, thiserror::Error)]
#[error("outer failure")]
pub(in crate::util) struct OuterError {
    #[source]
    pub(in crate::util) source: InnerError,
}
