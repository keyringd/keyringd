/// Reports unexpected infrastructure/UI failures at ERROR and returns the selected UI message.
/// Application failures use `ApplicationError::report_user_error` to classify their severity.
pub fn report_user_error(error: &impl std::fmt::Debug, message: &str) -> String {
    tracing::error!(?error, "{message}");
    message.to_owned()
}
