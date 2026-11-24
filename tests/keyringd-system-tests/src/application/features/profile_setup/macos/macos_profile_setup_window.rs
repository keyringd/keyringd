use super::MacosProfileSetupWizard;
use crate::application::features::profiles::macos::MacosProfilesWindow;
use crate::application::platform::macos::UiDriver;
use anyhow::Result;
pub struct MacosProfileSetupWindow<'a> {
    driver: UiDriver<'a>,
}
impl<'a> MacosProfileSetupWindow<'a> {
    pub(crate) fn new(driver: UiDriver<'a>) -> Self {
        Self { driver }
    }
}
impl super::super::ProfileSetupWindowDriver for MacosProfileSetupWindow<'_> {
    type Wizard<'a>
        = MacosProfileSetupWizard<'a>
    where
        Self: 'a;
    fn screenshot(&self) -> Result<std::path::PathBuf> {
        self.driver.capture(super::HEADING)
    }
    fn open(&mut self) -> Result<()> {
        MacosProfilesWindow::new(&self.driver)?.start_profile_creation()?;
        Ok(())
    }
    fn close(&mut self) -> Result<()> {
        self.driver.press("profile-setup.cancel")?;
        self.driver.wait_absent(super::HEADING)?;
        MacosProfilesWindow::new(&self.driver)?;
        Ok(())
    }
    fn wizard(&mut self) -> Result<Self::Wizard<'_>> {
        MacosProfileSetupWizard::current(&self.driver)
    }
}
