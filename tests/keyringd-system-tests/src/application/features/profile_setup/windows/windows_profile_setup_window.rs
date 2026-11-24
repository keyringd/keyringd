use super::WindowsProfileSetupWizard;
use crate::application::features::navigation::windows::UiApplication;
use crate::application::platform::windows::UiDriver;
use anyhow::Result;
pub struct WindowsProfileSetupWindow<'a> {
    driver: &'a UiDriver,
    pid: u32,
}
impl<'a> WindowsProfileSetupWindow<'a> {
    pub(crate) fn new(driver: &'a UiDriver, pid: u32) -> Self {
        Self { driver, pid }
    }
}
impl super::super::ProfileSetupWindowDriver for WindowsProfileSetupWindow<'_> {
    type Wizard<'a>
        = WindowsProfileSetupWizard<'a>
    where
        Self: 'a;
    fn screenshot(&self) -> Result<std::path::PathBuf> {
        self.driver
            .capture(&self.driver.window_named(self.pid, "Add SSH profile")?)
    }
    fn open(&mut self) -> Result<()> {
        UiApplication::new(self.driver, self.pid)
            .profiles_window()?
            .start_profile_creation()?;
        Ok(())
    }
    fn close(&mut self) -> Result<()> {
        self.driver.invoke(
            &self.driver.window_named(self.pid, "Add SSH profile")?,
            "profile-setup-cancel",
        )?;
        self.driver
            .wait_window_absent(self.pid, "Add SSH profile")?;
        self.driver
            .wait_window_enabled(&self.driver.window_named(self.pid, "Keyringd")?, true)
    }
    fn wizard(&mut self) -> Result<Self::Wizard<'_>> {
        self.driver.window_named(self.pid, "Add SSH profile")?;
        Ok(WindowsProfileSetupWizard::new(self.driver, self.pid))
    }
}
