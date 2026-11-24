use anyhow::Result;
use anyhow::ensure;
use keyringd_system_tests::application::SystemTestApplication;
use keyringd_system_tests::application::features::profile_setup::{
    PasskeyNotFoundPageDriver, ProfileSetupProgress,
};
use keyringd_system_tests::application::features::profile_setup::{
    ProfileSetupWindowDriver, ProfileSetupWizardDriver,
};
use keyringd_system_tests::environment::SystemTestEnvironment;

pub fn assert_content(progress: &ProfileSetupProgress) -> Result<()> {
    for expected in [
        "Passkey Not Found",
        "No Keyringd passkey was found. Choose how to continue.",
        "Try another device",
        "Create a new passkey instead",
    ] {
        ensure!(
            progress.heading == expected || progress.texts.iter().any(|text| text == expected),
            "missing recovery message {expected:?}: {progress:?}"
        );
    }
    ensure!(
        progress.continue_enabled == Some(true),
        "recovery Continue is unavailable: {progress:?}"
    );
    Ok(())
}

pub fn assert_visible(environment: &mut impl SystemTestEnvironment) -> Result<()> {
    super::progress::assert_when_ready(environment, assert_content)
}

pub fn start_new_creation(environment: &mut impl SystemTestEnvironment, name: &str) -> Result<()> {
    environment
        .application()
        .profile_setup_window()
        .wizard()?
        .passkey_not_found_page()?
        .continue_with_new_passkey()?;
    super::form::assert_content(environment, name, false)?;
    super::form::assert_message(
        environment,
        "Creating a new passkey will not restore access to servers you used with the previous passkey.",
    )
}
