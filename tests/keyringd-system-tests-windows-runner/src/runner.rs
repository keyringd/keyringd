use crate::RunnerOptions;
use anyhow::Result;
pub struct Runner;
impl Runner {
    pub fn execute(options: &RunnerOptions) -> Result<()> {
        #[cfg(windows)]
        {
            match &options.suite {
                crate::TestSuite::Windows => crate::execution::WindowsExecution::new(options).run(),
                crate::TestSuite::Wsl {
                    distribution,
                    package_directory,
                } => crate::execution::wsl::run(options, distribution, package_directory),
            }
        }
        #[cfg(not(windows))]
        {
            let _ = options;
            anyhow::bail!("Windows-host system tests require Windows")
        }
    }
}
