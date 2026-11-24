//! Shared Windows-host infrastructure for independent Windows UI and WSL suites.
#[cfg(windows)]
mod build;
#[cfg(windows)]
mod environment;
#[cfg(any(windows, test))]
mod execution;
mod runner;
mod runner_options;
mod test_suite;
#[cfg(windows)]
mod util;
pub use runner::Runner;
pub use runner_options::RunnerOptions;
pub use test_suite::TestSuite;
