use crate::environment::{AuthenticatorEnvironment, RunLock};
use crate::{RunnerOptions, TestbedExecutionRequest};
use anyhow::Result;

pub(crate) fn execute(
    options: &RunnerOptions,
    start_testbed: impl FnOnce(TestbedExecutionRequest) -> Result<()>,
) -> Result<()> {
    options.validate()?;
    let _run_lock = RunLock::acquire()?;

    let request = crate::build::prepare(options)?;
    let authenticator =
        keyringd_test_usbip::authenticator_server::build::AuthenticatorServerBuild::new(
            &options.workspace_root,
        )
        .prepare()?;
    let mut authenticator_server =
        AuthenticatorEnvironment::start(&authenticator, &request.runtime_directory())?;
    let testbed = start_testbed(request);
    let authenticator = authenticator_server.stop();
    crate::util::finish([
        ("Linux system testbed", testbed),
        ("virtual authenticator shutdown", authenticator),
    ])
}
