//! Compose the Windows environment and execute scenarios as the desktop user.
#[cfg(any(windows, test))]
mod authenticator_log_capture;
#[cfg(windows)]
mod case_runner;

mod windows_execution;
#[cfg(windows)]
pub(crate) mod wsl;
#[cfg(windows)]
pub(crate) use windows_execution::WindowsExecution;

#[cfg(windows)]
mod case_process;
