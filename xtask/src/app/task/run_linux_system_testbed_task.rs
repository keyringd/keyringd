use clap::Args;
use std::path::PathBuf;

#[derive(Args, Clone, Debug)]
pub(crate) struct RunLinuxSystemTestbedTask {
    #[arg(long)]
    pub(crate) distribution: String,
    #[arg(long)]
    pub(crate) workspace: PathBuf,
    #[arg(long)]
    pub(crate) package: Option<PathBuf>,
    #[arg(long)]
    pub(crate) test_executable: PathBuf,
    #[arg(long)]
    pub(crate) report_directory: PathBuf,
    #[arg(long)]
    pub(crate) image: PathBuf,
    #[arg(long)]
    pub(crate) machine: String,
    #[arg(long)]
    pub(crate) owner_uid: u32,
    #[arg(long)]
    pub(crate) owner_gid: u32,
    #[arg(long)]
    pub(crate) authenticator_control_url: String,
    #[arg(long)]
    pub(crate) filter: Option<String>,
}

impl RunLinuxSystemTestbedTask {
    pub(crate) fn exec(self) -> anyhow::Result<()> {
        #[cfg(target_os = "linux")]
        {
            use keyringd_system_tests_linux_runner::{Runner, TestbedExecutionRequest};
            Runner::execute_testbed(TestbedExecutionRequest {
                distribution: self.distribution.parse().map_err(anyhow::Error::msg)?,
                workspace: self.workspace,
                package: self.package,
                test_executable: self.test_executable,
                report_directory: self.report_directory,
                image: self.image,
                machine: self.machine,
                owner_uid: self.owner_uid,
                owner_gid: self.owner_gid,
                authenticator_control_url: self.authenticator_control_url,
                filter: self.filter,
            })
        }
        #[cfg(not(target_os = "linux"))]
        {
            anyhow::bail!("Linux system tests can only run on Linux")
        }
    }
}
