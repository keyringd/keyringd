use super::HEADING;
use crate::application::features::profile_setup::ProfileSetupQrViewDriver;
use crate::application::platform::macos::UiDriver;
use anyhow::Result;

pub struct ProfileSetupQrView<'a> {
    pub(super) driver: UiDriver<'a>,
}
impl ProfileSetupQrViewDriver for ProfileSetupQrView<'_> {
    fn content(&self) -> Result<super::super::ProfileSetupProgress> {
        self.driver
            .wait_for_fixture_state("pending Bluetooth request", |state| state.pending > 0)?;
        super::progress_observation::observe_progress(&self.driver)
    }
    fn read_hybrid_qr(&self, previous: Option<&str>) -> Result<(String, std::path::PathBuf)> {
        self.driver
            .wait_until("key-operation.qr", "to be visible", |element| {
                element.hidden == Some(false)
            })?;
        self.driver.wait_fresh_hybrid_qr(HEADING, previous)
    }
    fn press_use_security_key(&self) -> Result<()> {
        self.driver.wait_until(
            "profile-setup.use-security-key",
            "to be selectable",
            |element| element.enabled == Some(true),
        )?;
        self.driver.press("profile-setup.use-security-key")
    }
}
