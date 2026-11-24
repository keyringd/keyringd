use crate::core::{model::Workspace, service};
use clap::Args;
use std::path::PathBuf;

#[derive(Args, Clone, Debug)]
pub(crate) struct WslSystemTestTask {
    #[arg(long, default_value = "ubuntu", value_parser = ["ubuntu", "fedora"])]
    pub(crate) distribution: String,
    #[arg(long)]
    pub(crate) package_directory: Option<PathBuf>,
    #[arg(long)]
    pub(crate) filter: Option<String>,
}

use crate::core::model::SystemTestRequest;
impl From<WslSystemTestTask> for SystemTestRequest {
    fn from(options: WslSystemTestTask) -> Self {
        SystemTestRequest::Wsl {
            distribution: options.distribution,
            package_directory: options.package_directory,
            filter: options.filter,
        }
    }
}

impl WslSystemTestTask {
    pub(crate) fn exec(self, workspace: &Workspace) -> anyhow::Result<()> {
        service::system_tests::run(workspace, &self.into(), false)
    }

    pub(crate) fn exec_with_report(self, workspace: &Workspace) -> anyhow::Result<()> {
        service::system_tests::run(workspace, &self.into(), true)
    }
}

#[cfg(test)]
mod tests {
    use super::super::{TaskCommand, parse};

    keyringd_testkit::scenario! {
        group "parse" {
            case "Wsl system tests have a separate command and distribution" {
                verify "parse the WSL suite separately and preserve the requested Linux distribution" {
                    let cli = parse([
                        "xtask",
                        "wsl-system-test-with-report",
                        "--distribution",
                        "fedora",
                        "--filter",
                        "ssh_authentication",
                    ])
                    .unwrap();
                    assert!(matches!(cli, TaskCommand::WslSystemTestWithReport(_)));
                    assert!(parse(["xtask", "wsl-system-test", "--distribution", "windows"]).is_err());
                }
            }
        }
    }
}
