#[derive(Debug, thiserror::Error)]
pub(crate) enum PromptWindowError {
    #[error("could not create the prompt window")]
    Reactor(#[from] windows_reactor::Error),
    #[error("could not configure the prompt window")]
    Windows(#[from] windows::core::Error),
    #[error("could not locate the prompt window")]
    Locator(#[from] std::io::Error),
}
