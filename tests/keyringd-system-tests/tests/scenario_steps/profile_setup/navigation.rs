use anyhow::Result;
use keyringd_system_tests::application::SystemTestApplication;
use keyringd_system_tests::application::features::profile_setup::{
    ProfileSetupWindowDriver, ProfileSetupWizardDriver,
};
use keyringd_system_tests::environment::SystemTestEnvironment;

pub fn back_to_form(
    environment: &mut impl SystemTestEnvironment,
    name: &str,
    existing: bool,
) -> Result<()> {
    environment
        .application()
        .profile_setup_window()
        .wizard()?
        .back()?;
    super::form::assert_message(environment, "Setup cancelled. You can try again.")?;
    super::form::assert_content(environment, name, existing)
}

pub fn cancel(environment: &mut impl SystemTestEnvironment) -> Result<()> {
    environment.application().profile_setup_window().close()?;
    #[cfg(windows)]
    environment
        .application()
        .wait_for_fido_operations_to_finish()?;
    Ok(())
}
