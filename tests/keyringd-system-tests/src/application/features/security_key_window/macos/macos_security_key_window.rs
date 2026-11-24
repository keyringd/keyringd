use super::super::SecurityKeyWindowDriver;
use crate::application::platform::macos::UiDriver;
use crate::environment::macos::TestApp;
use anyhow::Result;
use std::path::PathBuf;
const HEADING: &str = "Verify your identity on your security key";
pub struct MacosSecurityKeyWindow<'a> {
    driver: UiDriver<'a>,
}
impl<'a> MacosSecurityKeyWindow<'a> {
    pub(crate) fn new(app: &'a TestApp) -> Self {
        Self {
            driver: UiDriver::new(app).in_window_with_value_containing(HEADING),
        }
    }
}
impl SecurityKeyWindowDriver for MacosSecurityKeyWindow<'_> {
    fn wait_open(&self) -> Result<()> {
        self.driver.wait_valued(HEADING)?;
        Ok(())
    }
    fn screenshot(&self) -> Result<PathBuf> {
        self.driver.wait_valued(HEADING)?;
        self.driver.capture_current_window()
    }
    fn wait_closed(&self) -> Result<()> {
        self.driver.wait_value_absent(HEADING)
    }
}
