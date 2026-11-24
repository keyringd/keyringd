use super::super::PinPromptWindowDriver;
use crate::application::platform::macos::UiDriver;
use crate::environment::macos::TestApp;
use anyhow::Result;
use std::path::PathBuf;
pub struct MacosPinPromptWindow<'a> {
    driver: UiDriver<'a>,
}
impl<'a> MacosPinPromptWindow<'a> {
    pub(crate) fn new(app: &'a TestApp) -> Self {
        Self {
            driver: UiDriver::new(app).in_window("key-operation.pin"),
        }
    }
    pub fn validation_message(&self) -> Result<Option<String>> {
        Ok(self
            .driver
            .find_element("key-operation.pin-validation")?
            .and_then(|e| e.value))
    }
}
impl PinPromptWindowDriver for MacosPinPromptWindow<'_> {
    fn screenshot(&self) -> Result<PathBuf> {
        self.driver.wait_for("key-operation.pin")?;
        self.driver.capture_current_window()
    }
    fn enter_pin(&self, pin: &str) -> Result<()> {
        self.driver.wait_for("key-operation.pin")?;
        self.driver.enter_text("key-operation.pin", pin)?;
        Ok(())
    }
    fn submit(&self) -> Result<()> {
        self.driver
            .wait_until("key-operation.pin-submit", "to accept the PIN", |e| {
                e.enabled == Some(true)
            })?;
        self.driver.press("key-operation.pin-submit")
    }
    fn wait_closed(&self) -> Result<()> {
        self.driver.wait_absent("key-operation.pin")
    }
}
