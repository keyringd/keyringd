use super::super::QrPromptWindowDriver;
use crate::application::platform::windows::UiDriver;
use anyhow::{Result, ensure};
use std::path::PathBuf;
use windows::Win32::UI::Accessibility::IUIAutomationElement;
pub struct WindowsQrPromptWindow<'a> {
    driver: &'a UiDriver,
    pid: u32,
}
impl<'a> WindowsQrPromptWindow<'a> {
    pub(crate) fn new(driver: &'a UiDriver, pid: u32) -> Self {
        Self { driver, pid }
    }
    pub fn cancel(&self) -> Result<()> {
        self.driver.invoke_named(&self.window()?, "Cancel")
    }
    fn current(&self) -> Result<Option<IUIAutomationElement>> {
        let mut found = None;
        for handle in
            crate::application::platform::windows::window_locator::visible_windows(self.pid)?
        {
            let window = self.driver.element_from_handle(handle.0 as usize)?;
            if self
                .driver
                .visible_element_by_id(&window, "qr-prompt-window")?
                .is_some()
            {
                ensure!(
                    found.is_none(),
                    "more than one QR prompt window is visible for PID {}",
                    self.pid
                );
                found = Some(window);
            }
        }
        Ok(found)
    }
    fn window(&self) -> Result<IUIAutomationElement> {
        self.driver
            .wait("QR prompt window", || self.current())
            .map_err(|error| error.context(self.driver.diagnose_windows(self.pid)))
    }
}
impl QrPromptWindowDriver for WindowsQrPromptWindow<'_> {
    fn read_hybrid_qr(&self) -> Result<(String, PathBuf)> {
        self.driver.read_hybrid_qr(&self.window()?, None)
    }
    fn security_key_enabled(&self) -> Result<bool> {
        let window = self.window()?;
        self.driver
            .wait_visible_named(&window, "Use security key…")?;
        Ok(unsafe {
            self.driver
                .element_named(&window, "Use security key…")?
                .CurrentIsEnabled()?
                .as_bool()
        })
    }
    fn press_use_security_key(&self) -> Result<()> {
        self.driver
            .invoke_named(&self.window()?, "Use security key…")
    }
    fn screenshot(&self) -> Result<PathBuf> {
        self.driver.capture(&self.window()?)
    }
    fn wait_closed(&self) -> Result<()> {
        self.driver.wait("QR prompt window to close", || {
            Ok(self.current()?.is_none().then_some(()))
        })
    }
}
