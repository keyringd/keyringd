use crate::application::platform::macos::UiDriver;
use anyhow::Result;
use std::path::PathBuf;
const CONFIGURE: &str = "getting-started.configure";
const STATUS: &str = "getting-started.status";
pub struct MacosSshClientConfigurationWindow<'a> {
    driver: UiDriver<'a>,
}
impl<'a> MacosSshClientConfigurationWindow<'a> {
    pub fn open(driver: &UiDriver<'a>) -> Result<Self> {
        driver
            .in_window("profiles.configure-ssh-clients")
            .press("profiles.configure-ssh-clients")?;
        let page = Self {
            driver: driver.in_window(CONFIGURE),
        };
        page.driver.wait_for(CONFIGURE)?;
        Ok(page)
    }
    pub fn open_confirmation(
        &self,
    ) -> Result<super::MacosSshClientConfigurationConfirmationWindow<'a>> {
        self.driver.press(CONFIGURE)?;
        super::MacosSshClientConfigurationConfirmationWindow::wait_for(&self.driver)
    }
    pub fn capture_screenshot(&self, name: &str) -> anyhow::Result<()> {
        keyringd_testkit::record_image(name, self.screenshot()?)
    }
    pub fn screenshot(&self) -> Result<PathBuf> {
        self.driver.capture(STATUS)
    }
    pub fn close(&self) -> Result<()> {
        self.driver.close_window(CONFIGURE)?;
        self.driver.wait_absent(CONFIGURE)
    }
}
