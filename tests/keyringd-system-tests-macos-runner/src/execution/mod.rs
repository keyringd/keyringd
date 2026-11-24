#![cfg_attr(not(target_os = "macos"), allow(dead_code))]

mod case_process;
mod case_runner;

mod macos_execution;
mod process_outcome;
#[cfg(target_os = "macos")]
pub(crate) use macos_execution::MacosExecution;

use process_outcome::ProcessOutcome;

mod case_execution;
