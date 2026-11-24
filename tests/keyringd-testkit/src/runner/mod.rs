//! OS-independent parsing and result verification for test runners.
//! Callers own commands, permissions, processes, and cleanup.
mod cargo_build_artifacts;
mod scenario_results;
mod scenario_selection;
mod stage_failures;

pub use cargo_build_artifacts::CargoBuildArtifacts;
pub use scenario_results::ScenarioResults;
pub use scenario_selection::ScenarioSelection;
pub use stage_failures::StageFailures;

mod command_observation;
mod report_run_metadata;
mod tool_execution_metadata;
#[cfg(windows)]
mod windows_run_information;
pub use report_run_metadata::ReportRunMetadata;
pub use tool_execution_metadata::ToolExecutionMetadata;
