//! Builds and supervises the macOS system test environment and scenario processes.
#[cfg(any(target_os = "macos", all(unix, feature = "test-support")))]
mod build;
#[cfg(any(target_os = "macos", all(unix, feature = "test-support")))]
mod environment;
#[cfg(any(target_os = "macos", all(unix, feature = "test-support")))]
mod execution;
mod runner;
mod runner_options;

pub use runner::Runner;
pub use runner_options::RunnerOptions;
