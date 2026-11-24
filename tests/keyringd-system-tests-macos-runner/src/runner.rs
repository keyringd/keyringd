use crate::RunnerOptions;
use anyhow::Result;

/// Entry point for building and running macOS UI scenarios.
pub struct Runner;
impl Runner {
    pub fn execute(options: &RunnerOptions) -> Result<()> {
        #[cfg(target_os = "macos")]
        {
            anyhow::ensure!(
                options.case_timeout_seconds > 0,
                "case timeout must be positive"
            );
            let workspace = crate::environment::Workspace::new(options.workspace_root.clone())?;
            crate::execution::MacosExecution::new(&workspace, options).run()
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = options;
            anyhow::bail!("macOS system tests can only run on macOS")
        }
    }
}
