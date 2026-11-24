use crate::{RunnerOptions, TestbedExecutionRequest};
use anyhow::Result;

/// Entry point for Linux system test execution, including the privileged testbed boundary.
pub struct Runner;
impl Runner {
    pub fn execute(
        options: &RunnerOptions,
        start_testbed: impl FnOnce(TestbedExecutionRequest) -> Result<()>,
    ) -> Result<()> {
        crate::execution::execute(options, start_testbed)
    }
    pub fn execute_testbed(request: TestbedExecutionRequest) -> Result<()> {
        crate::environment::LinuxTestbed::execute(request)
    }
}
