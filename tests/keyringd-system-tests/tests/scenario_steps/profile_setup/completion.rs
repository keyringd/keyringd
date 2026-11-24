use anyhow::Result;
use keyringd_system_tests::application::SystemTestApplication;
use keyringd_system_tests::application::features::profile_setup::{
    ProfileSetupWindowDriver, ProfileSetupWizardDriver,
};
use keyringd_system_tests::environment::SystemTestEnvironment;

pub fn wait_for(environment: &mut impl SystemTestEnvironment) -> Result<()> {
    environment
        .application()
        .profile_setup_window()
        .wizard()?
        .completion_page()?;
    Ok(())
}
