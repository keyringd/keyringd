use crate::scenario_steps::waiting::until;
use anyhow::Result;
use keyringd_system_tests::application::features::profile_setup::ProfileSetupWaitingViewDriver;

pub fn assert_uv_prompt(view: &impl ProfileSetupWaitingViewDriver) -> Result<()> {
    wait_for_waiting_text(view, &["Verify your identity on your security key"])
}
pub fn assert_creation_approval(view: &impl ProfileSetupWaitingViewDriver) -> Result<()> {
    wait_for_waiting_text(view, &["Approve Passkey creation"])
}
pub fn wait_for_key_selection(view: &impl ProfileSetupWaitingViewDriver) -> Result<()> {
    wait_for_waiting_text(view, &["Choose your security key"])
}
pub fn wait_for_key_prompt(view: &impl ProfileSetupWaitingViewDriver) -> Result<()> {
    wait_for_waiting_text(
        view,
        &[
            "Choose your security key",
            "Approve Passkey creation",
            "Authorize this computer",
            "Find the Passkey",
            "Verify the Passkey",
            "Approve on your security key",
            "Verify your identity on your security key",
        ],
    )
}
fn wait_for_waiting_text(
    view: &impl ProfileSetupWaitingViewDriver,
    expected: &[&str],
) -> Result<()> {
    until(&format!("security key prompt {expected:?}"), || {
        Ok(view
            .content()?
            .iter()
            .any(|text| expected.contains(&text.as_str())))
    })
}
