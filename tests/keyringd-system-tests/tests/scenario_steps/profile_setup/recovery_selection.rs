use anyhow::Result;
use keyringd_system_tests::application::SystemTestApplication;
#[cfg(unix)]
use keyringd_system_tests::application::features::profile_setup::PasskeySelectionPageDriver;
#[cfg(unix)]
use keyringd_system_tests::application::features::profile_setup::ProfileSetupStep;
use keyringd_system_tests::application::features::profile_setup::ProfileSetupWindowDriver;
use keyringd_system_tests::application::features::profile_setup::ProfileSetupWizardDriver;
#[cfg(unix)]
use keyringd_system_tests::application::features::profile_setup::{
    ProfileSetupInteractionPageDriver, ProfileSetupQrViewDriver,
};
#[cfg(unix)]
use keyringd_system_tests::authenticator::ApprovalMode;
use keyringd_system_tests::authenticator::UsbSecurityKey;
use keyringd_system_tests::environment::SystemTestEnvironment;
#[cfg(unix)]
use keyringd_test_usbip::authenticator_server::client::ConsentKind;

#[cfg(unix)]
fn approve_discovery(
    environment: &mut impl SystemTestEnvironment,
    key: &UsbSecurityKey,
) -> Result<()> {
    let request = key
        .authenticator()
        .wait_for_any_request_approving_selection(&[
            ConsentKind::PinUvAuthToken,
            ConsentKind::GetAssertion,
        ])?;
    super::security_key_prompt::wait_for_key_prompt(
        &environment
            .application()
            .profile_setup_window()
            .wizard()?
            .interaction_page()?
            .waiting_view()?,
    )?;
    key.authenticator().approve(request.id)
}

#[cfg(unix)]
pub fn discover_credentials(
    environment: &mut impl SystemTestEnvironment,
    key: &UsbSecurityKey,
    name: &str,
) -> Result<Vec<String>> {
    key.authenticator()
        .set_approval_mode(ApprovalMode::Manual)?;
    super::form::submit_recovery(environment, name)?;
    #[cfg(unix)]
    {
        super::transport_choices::assert_recovery(environment)?;
        environment
            .application()
            .profile_setup_window()
            .wizard()?
            .interaction_page()?
            .qr_view()?
            .press_use_security_key()?;
        approve_discovery(environment, key)?;
    }
    environment
        .application()
        .profile_setup_window()
        .wizard()?
        .passkey_selection_page()?
        .candidate_labels()
}

#[cfg(unix)]
pub fn approve_selected(
    environment: &mut impl SystemTestEnvironment,
    key: &UsbSecurityKey,
) -> Result<()> {
    #[cfg(unix)]
    {
        super::security_key::approve_step(environment, key, ProfileSetupStep::Completed)?;
        key.authenticator()
            .set_approval_mode(ApprovalMode::Automatic)?;
    }
    Ok(())
}

pub fn discover_missing_passkey(
    environment: &mut impl SystemTestEnvironment,
    key: &UsbSecurityKey,
    name: &str,
) -> Result<()> {
    #[cfg(windows)]
    let _ = key;
    super::form::submit_recovery(environment, name)?;
    #[cfg(unix)]
    {
        super::transport_choices::assert_recovery(environment)?;
        environment
            .application()
            .profile_setup_window()
            .wizard()?
            .interaction_page()?
            .qr_view()?
            .press_use_security_key()?;
        approve_discovery(environment, key)?;
    }
    super::passkey_not_found::assert_visible(environment)
}

pub fn resume(environment: &mut impl SystemTestEnvironment) -> Result<()> {
    #[cfg(unix)]
    {
        super::transport_choices::assert_recovery(environment)?;
        environment
            .application()
            .profile_setup_window()
            .wizard()?
            .interaction_page()?
            .qr_view()?
            .press_use_security_key()?;
    }
    #[cfg(windows)]
    let _ = environment;
    Ok(())
}

#[cfg(unix)]
pub fn discover_on_selected_key(
    environment: &mut impl SystemTestEnvironment,
    selected: &UsbSecurityKey,
    other: &UsbSecurityKey,
    name: &str,
) -> Result<()> {
    super::form::submit_recovery(environment, name)?;
    super::transport_choices::assert_recovery(environment)?;
    environment
        .application()
        .profile_setup_window()
        .wizard()?
        .interaction_page()?
        .qr_view()?
        .press_use_security_key()?;
    super::security_key::select(environment, selected, other)?;
    let request = selected
        .authenticator()
        .wait_for_request_approving_selection(ConsentKind::PinUvAuthToken)?;
    selected.authenticator().approve(request.id)?;
    super::passkey_not_found::assert_visible(environment)?;
    Ok(())
}
