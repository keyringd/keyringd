use crate::scenario_steps::waiting::until;
use anyhow::{Result, ensure};
use keyringd_system_tests::application::features::profile_setup::ProfileSetupQrViewDriver;

pub fn assert_bluetooth_disabled(view: &impl ProfileSetupQrViewDriver) -> Result<()> {
    assert_bluetooth_unavailable(
        view,
        "Bluetooth is off. Turn it on to use a passkey on your phone or tablet.",
    )
}
pub fn assert_no_bluetooth_adapter(view: &impl ProfileSetupQrViewDriver) -> Result<()> {
    assert_bluetooth_unavailable(
        view,
        "No Bluetooth adapter was found. Connect or enable one to use a passkey on your phone or tablet.",
    )
}
fn assert_bluetooth_unavailable(view: &impl ProfileSetupQrViewDriver, message: &str) -> Result<()> {
    let expected = [
        "Can't connect to your phone or tablet",
        "Bluetooth must be enabled on both devices.",
        message,
        "Use a security key instead?",
        "You can also use a connected USB security key.",
    ];
    until(&format!("Bluetooth unavailable: {message}"), || {
        let content = view.content()?;
        Ok(content.security_key_enabled == Some(true)
            && expected
                .iter()
                .all(|text| content.texts.iter().any(|actual| actual == text)))
    })
}

use keyringd_system_tests::application::features::profile_setup::{
    ProfileSetupProgress, ProfileSetupStep,
};
use keyringd_system_tests::environment::SystemTestEnvironment;

pub fn assert_content(progress: &ProfileSetupProgress, step: ProfileSetupStep) -> Result<()> {
    super::progress::assert_step(progress, step)?;
    let instruction = match step {
        ProfileSetupStep::CreatePasskey => {
            "Scan with the phone or tablet where you want to create the Passkey."
        }
        ProfileSetupStep::FindPasskey => {
            "Scan with the phone or tablet that contains your existing passkey."
        }
        _ => anyhow::bail!("{step:?} is not an authenticator choice page"),
    };
    for expected in [
        instruction,
        "Bluetooth must be enabled on both devices.",
        "Use a security key instead?",
        "You can also use a connected USB security key.",
    ] {
        ensure!(
            progress.texts.iter().any(|text| text == expected),
            "missing setup instruction {expected:?}: {progress:?}"
        );
    }
    ensure!(
        progress.security_key_enabled == Some(true),
        "Security Key choice is unavailable: {progress:?}"
    );
    Ok(())
}

pub fn assert_creation(environment: &mut impl SystemTestEnvironment) -> Result<()> {
    super::progress::assert_when_ready(environment, |progress| {
        assert_content(progress, ProfileSetupStep::CreatePasskey)
    })
}

pub fn assert_recovery(environment: &mut impl SystemTestEnvironment) -> Result<()> {
    super::progress::assert_when_ready(environment, |progress| {
        assert_content(progress, ProfileSetupStep::FindPasskey)
    })
}
