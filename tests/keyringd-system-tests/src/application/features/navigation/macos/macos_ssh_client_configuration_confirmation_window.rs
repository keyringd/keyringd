use crate::application::platform::macos::UiDriver;
use anyhow::Result;
use std::path::PathBuf;

pub struct MacosSshClientConfigurationConfirmationWindow<'a> {
    driver: UiDriver<'a>,
}
impl<'a> MacosSshClientConfigurationConfirmationWindow<'a> {
    pub(super) fn wait_for(driver: &UiDriver<'a>) -> Result<Self> {
        let window = Self {
            driver: driver.in_sheet("getting-started.configure"),
        };
        window
            .driver
            .wait_for("getting-started.confirmation.confirm")?;
        Ok(window)
    }
    pub fn screenshot(&self) -> Result<PathBuf> {
        self.driver.capture_current_window()
    }
    pub fn capture_screenshot(&self, name: &str) -> Result<()> {
        keyringd_testkit::record_image(name, self.screenshot()?)
    }
    pub fn cancel(&self) -> Result<()> {
        self.driver.press("getting-started.confirmation.cancel")?;
        self.driver
            .wait_absent("getting-started.confirmation.confirm")
    }

    pub fn confirm(&self) -> Result<()> {
        let parent = self.driver.in_window("getting-started.configure");
        let previous = parent.element("getting-started.configure")?.title;
        self.driver.press("getting-started.confirmation.confirm")?;
        self.driver
            .wait_absent("getting-started.confirmation.confirm")?;
        parent.wait_until(
            "getting-started.configure",
            "to finish changing the SSH agent",
            |element| element.enabled == Some(true) && element.title != previous,
        )?;
        Ok(())
    }
}
