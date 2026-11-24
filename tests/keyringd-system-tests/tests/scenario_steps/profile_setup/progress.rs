use anyhow::{Result, ensure};
use keyringd_system_tests::application::SystemTestApplication;
use keyringd_system_tests::application::features::profile_setup::ProfileSetupWindowDriver;
use keyringd_system_tests::application::features::profile_setup::{
    ProfileSetupProgress, ProfileSetupStep, ProfileSetupWizardDriver,
};
use keyringd_system_tests::environment::SystemTestEnvironment;
use std::time::{Duration, Instant};

pub fn assert_step(progress: &ProfileSetupProgress, step: ProfileSetupStep) -> Result<()> {
    ensure!(
        progress.heading == step.title() || progress.texts.iter().any(|text| text == step.title()),
        "expected setup step {step:?}: {progress:?}"
    );
    Ok(())
}

pub fn is_step_visible(
    environment: &mut impl SystemTestEnvironment,
    step: ProfileSetupStep,
) -> Result<bool> {
    let progress = environment
        .application()
        .profile_setup_window()
        .wizard()?
        .observe()?;
    Ok(assert_step(&progress, step).is_ok())
}

pub fn wait_for_step(
    environment: &mut impl SystemTestEnvironment,
    step: ProfileSetupStep,
) -> Result<()> {
    assert_when_ready(environment, |progress| assert_step(progress, step))
}

pub(super) fn assert_when_ready(
    environment: &mut impl SystemTestEnvironment,
    check: impl Fn(&ProfileSetupProgress) -> Result<()>,
) -> Result<()> {
    let deadline = Instant::now() + Duration::from_secs(60);
    let mut last_mismatch = None;
    loop {
        let progress = environment
            .application()
            .profile_setup_window()
            .wizard()?
            .observe()
            .map_err(|error| match &last_mismatch {
                Some(previous) => error.context(format!("last setup observation: {previous:#}")),
                None => error,
            })?;
        match check(&progress) {
            Ok(()) => return Ok(()),
            Err(error) if Instant::now() >= deadline => return Err(error),
            Err(error) => {
                last_mismatch = Some(error);
                std::thread::sleep(Duration::from_millis(25));
            }
        }
    }
}
