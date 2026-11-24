use super::SetVersionTask;
use crate::core::service;
use clap::Subcommand;

use super::BuildTask;
use super::DistTask;
use super::DocsTask;
use super::HostTestsWithReportTask;
use super::LinuxSystemTestTask;
use super::MacosSystemTestTask;
use super::TestReportTask;
use super::TestWithReportTask;
use super::WindowsSystemTestTask;
use super::WslSystemTestTask;

#[derive(Subcommand)]
pub(crate) enum TaskCommand {
    /// Builds this host OS's applications (Linux also builds the WSL proxy).
    Build(BuildTask),

    /// Builds instrumented package inputs for system tests run on another host.
    CoverageBuild(super::CoverageBuildTask),

    /// Merges coverage artifacts and renders one combined HTML report.
    CoverageReport(super::CoverageReportTask),

    /// Builds or previews the Roq documentation website.
    Docs {
        #[command(subcommand)]
        task: DocsTask,
    },

    /// Runs the Linux system tests without rendering an HTML report.
    LinuxSystemTest(LinuxSystemTestTask),

    /// Runs Cargo tests and then renders their HTML report.
    TestWithReport(TestWithReportTask),

    /// Runs the Linux system tests and then renders their HTML report.
    LinuxSystemTestWithReport(LinuxSystemTestTask),

    /// Runs the macOS system tests without rendering an HTML report.
    MacosSystemTest(MacosSystemTestTask),

    /// Runs the macOS system tests and then renders their HTML report.
    MacosSystemTestWithReport(MacosSystemTestTask),

    /// Runs the Windows system tests without rendering an HTML report.
    WindowsSystemTest(WindowsSystemTestTask),

    /// Runs the Windows system tests and then renders their HTML report.
    WindowsSystemTestWithReport(WindowsSystemTestTask),

    /// Tests installed WSL packages without a Windows product installation.
    WslSystemTest(WslSystemTestTask),

    /// Runs WSL system tests and renders their HTML report.
    WslSystemTestWithReport(WslSystemTestTask),

    /// Runs Cargo tests and this host's system tests in one combined report.
    HostTestsWithReport(HostTestsWithReportTask),

    /// Generates an HTML report site from recorded test results.
    TestReport(TestReportTask),

    /// Builds distribution artifacts (deb, rpm, msi, or Windows runtime) from a pre-built binary.
    Dist(DistTask),

    /// Sets the shared workspace release version and refreshes Cargo.lock.
    SetVersion(SetVersionTask),

    /// Advances the shared workspace version to the next patch and refreshes Cargo.lock.
    BumpVersion(super::BumpVersionTask),

    /// Runs the prepared Linux system testbed (internal task).
    #[command(hide = true)]
    RunLinuxSystemTestbed(super::RunLinuxSystemTestbedTask),
}

impl TaskCommand {
    pub(crate) fn exec_with_coverage(&self) -> anyhow::Result<()> {
        let (suite, os) = match self {
            Self::TestWithReport(_) => ("ut".to_owned(), std::env::consts::OS.to_owned()),
            Self::LinuxSystemTest(task) | Self::LinuxSystemTestWithReport(task) => {
                (format!("system-{}", task.distribution), "linux".to_owned())
            }
            Self::MacosSystemTest(_) | Self::MacosSystemTestWithReport(_) => {
                ("system".to_owned(), "macos".to_owned())
            }
            Self::WindowsSystemTest(_) | Self::WindowsSystemTestWithReport(_) => {
                ("system".to_owned(), "windows".to_owned())
            }
            Self::WslSystemTest(task) | Self::WslSystemTestWithReport(task) => {
                (format!("system-{}", task.distribution), "wsl".to_owned())
            }
            Self::HostTestsWithReport(_) => {
                ("host-tests".to_owned(), std::env::consts::OS.to_owned())
            }
            _ => anyhow::bail!("--coverage is supported only by test commands"),
        };
        service::coverage::run(&suite, &os)
    }

    pub(crate) fn exec(self) -> anyhow::Result<()> {
        let root = match &self {
            Self::Dist(task) => task.workspace.clone(),
            _ => None,
        };
        let workspace = service::workspace::load(root)?;
        match self {
            Self::Build(task) => task.exec(&workspace),
            Self::CoverageBuild(task) => task.exec(&workspace),
            Self::CoverageReport(task) => task.exec(&workspace),
            Self::Docs { task } => task.exec(&workspace),
            Self::Dist(task) => task.exec(&workspace),
            Self::SetVersion(task) => task.exec(&workspace),
            Self::BumpVersion(task) => task.exec(&workspace),
            Self::LinuxSystemTest(task) => task.exec(&workspace),
            Self::MacosSystemTest(task) => task.exec(&workspace),
            Self::WindowsSystemTest(task) => task.exec(&workspace),
            Self::WslSystemTest(task) => task.exec(&workspace),
            Self::LinuxSystemTestWithReport(task) => task.exec_with_report(&workspace),
            Self::MacosSystemTestWithReport(task) => task.exec_with_report(&workspace),
            Self::WindowsSystemTestWithReport(task) => task.exec_with_report(&workspace),
            Self::WslSystemTestWithReport(task) => task.exec_with_report(&workspace),
            Self::TestWithReport(task) => task.exec(&workspace),
            Self::HostTestsWithReport(task) => task.exec(&workspace),
            Self::TestReport(task) => task.exec(),
            Self::RunLinuxSystemTestbed(task) => task.exec(),
        }
    }
}

#[cfg(test)]
pub(super) fn parse<const N: usize>(arguments: [&str; N]) -> Result<TaskCommand, clap::Error> {
    use clap::FromArgMatches;
    let matches =
        TaskCommand::augment_subcommands(clap::Command::new("xtask").subcommand_required(true))
            .try_get_matches_from(arguments)?;
    TaskCommand::from_arg_matches(&matches)
}
