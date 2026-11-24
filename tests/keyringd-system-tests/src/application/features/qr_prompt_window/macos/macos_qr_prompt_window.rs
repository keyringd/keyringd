use super::super::QrPromptWindowDriver;
use crate::application::platform::macos::UiDriver;
use anyhow::Result;
use std::path::PathBuf;
const PROMPT: &str = "key-operation.prompt";
const USE_SECURITY_KEY: &str = "key-operation.use-security-key";
pub struct MacosQrPromptWindow<'a> {
    driver: UiDriver<'a>,
}
impl<'a> MacosQrPromptWindow<'a> {
    pub(crate) fn new(driver: &UiDriver<'a>) -> Self {
        Self {
            driver: driver.in_window(PROMPT),
        }
    }
}
impl QrPromptWindowDriver for MacosQrPromptWindow<'_> {
    fn read_hybrid_qr(&self) -> Result<(String, PathBuf)> {
        self.driver.wait_for("key-operation.qr")?;
        self.driver.wait_hybrid_qr(PROMPT)
    }
    fn security_key_enabled(&self) -> Result<bool> {
        Ok(self.driver.wait_for(USE_SECURITY_KEY)?.enabled == Some(true))
    }
    fn press_use_security_key(&self) -> Result<()> {
        self.driver
            .wait_valued("Bluetooth must be enabled on both devices.")?;
        self.driver
            .wait_until(USE_SECURITY_KEY, "to be selectable", |e| {
                e.enabled == Some(true)
            })?;
        self.driver.press(USE_SECURITY_KEY)
    }
    fn screenshot(&self) -> Result<PathBuf> {
        self.driver.wait_for(USE_SECURITY_KEY)?;
        self.driver.capture(PROMPT)
    }
    fn wait_closed(&self) -> Result<()> {
        self.driver.wait_absent(PROMPT)
    }
}
