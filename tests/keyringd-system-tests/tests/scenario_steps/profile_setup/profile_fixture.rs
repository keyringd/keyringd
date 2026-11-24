use anyhow::Result;
use keyringd_system_tests::application::ApplicationConfiguration;
use keyringd_system_tests::application::SystemTestApplication;
use keyringd_system_tests::application::features::profile_setup::ProfileSetupWindowDriver;
use keyringd_system_tests::application::features::profile_setup::{
    CompletionPageDriver, ProfileSetupInteractionPageDriver, ProfileSetupQrViewDriver,
};
use keyringd_system_tests::authenticator::UsbSecurityKey;
use keyringd_system_tests::environment::SystemTestEnvironment;

use keyringd_system_tests::application::features::profile_setup::ProfileSetupWizardDriver;

/// Shared fixture preparation through the same UI actions on every platform.
pub fn create<D>(names: &[&str]) -> Result<(D, UsbSecurityKey)>
where
    D: SystemTestEnvironment,
{
    let mut environment = D::new()?;
    let key = environment
        .authenticators()
        .create_security_key(Default::default())?;
    environment
        .application()
        .start(ApplicationConfiguration::default())?;
    environment.application().profile_setup_window().open()?;
    if names.is_empty() {
        environment.application().profile_setup_window().close()?;
    }
    for (index, name) in names.iter().enumerate() {
        if index != 0 {
            environment.application().profile_setup_window().open()?;
        }
        super::form::submit_creation(&mut environment, name)?;
        environment
            .application()
            .profile_setup_window()
            .wizard()?
            .interaction_page()?
            .qr_view()?
            .press_use_security_key()?;
        environment
            .application()
            .profile_setup_window()
            .wizard()?
            .completion_page()?
            .done()?;
        crate::scenario_steps::profile_management::profile_list::assert_visible(
            &environment.application().profiles_window()?,
            name.trim(),
        )?;
    }
    Ok((environment, key))
}

#[cfg(unix)]
use keyringd_system_tests::authenticator::ApprovalMode;

pub fn create_recovery_source(
    environment: &mut impl SystemTestEnvironment,
    key: &UsbSecurityKey,
    name: &str,
) -> Result<()> {
    #[cfg(windows)]
    let _ = key;
    #[cfg(unix)]
    key.authenticator()
        .set_approval_mode(ApprovalMode::Automatic)?;
    super::form::submit_creation(environment, name)?;
    #[cfg(unix)]
    environment
        .application()
        .profile_setup_window()
        .wizard()?
        .interaction_page()?
        .qr_view()?
        .press_use_security_key()?;
    environment
        .application()
        .profile_setup_window()
        .wizard()?
        .completion_page()?
        .done()
}
