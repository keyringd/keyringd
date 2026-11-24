use crate::scenario_steps::waiting::until;
use anyhow::Result;
use keyringd_system_tests::application::features::profiles::ProfilesWindowDriver;

pub fn assert_visible(window: &impl ProfilesWindowDriver, name: &str) -> Result<()> {
    until(&format!("profile {name:?} to be visible"), || {
        window.profile_visible(name)
    })
}
pub fn assert_absent(window: &impl ProfilesWindowDriver, name: &str) -> Result<()> {
    until(&format!("profile {name:?} to be absent"), || {
        Ok(!window.profile_present(name)?)
    })
}
pub fn assert_enabled(window: &impl ProfilesWindowDriver, name: &str, enabled: bool) -> Result<()> {
    until(&format!("profile {name:?} enabled={enabled}"), || {
        Ok(window.profile_enabled(name)? == enabled)
    })
}
pub fn assert_above(window: &impl ProfilesWindowDriver, upper: &str, lower: &str) -> Result<()> {
    until(&format!("profile {upper:?} above {lower:?}"), || {
        window.profile_is_above(upper, lower)
    })
}
