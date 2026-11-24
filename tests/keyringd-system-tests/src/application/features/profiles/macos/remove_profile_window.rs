use crate::application::platform::macos::UiDriver;
use anyhow::Result;
pub struct RemoveProfileWindow<'a> {
    driver: UiDriver<'a>,
}
impl<'a> RemoveProfileWindow<'a> {
    pub(super) fn wait_for(driver: &UiDriver<'a>) -> Result<Self> {
        let driver = driver.in_sheet("profiles.configure-ssh-clients");
        driver.wait_for("remove-profile.confirm")?;
        Ok(Self { driver })
    }
}
impl crate::application::features::profiles::RemoveProfileWindowDriver for RemoveProfileWindow<'_> {
    fn screenshot(&self) -> Result<std::path::PathBuf> {
        self.driver.capture_current_window()
    }
    fn confirm(&mut self) -> Result<()> {
        self.driver.press("remove-profile.confirm")?;
        self.driver.wait_absent("remove-profile.confirm")
    }
}
