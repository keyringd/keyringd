use crate::application::platform::windows::UiDriver;
use anyhow::Result;
use windows::Win32::UI::Accessibility::IUIAutomationElement;

pub struct WindowsSshClientConfigurationConfirmationWindow<'a> {
    pub(super) driver: &'a UiDriver,
    pub(super) window: IUIAutomationElement,
    pub(super) pid: u32,
}
impl<'a> WindowsSshClientConfigurationConfirmationWindow<'a> {
    pub fn cancel(&self) -> Result<()> {
        self.driver.invoke_named(&self.window, "Cancel")?;
        self.driver.wait_window_absent(self.pid, "SSH_AUTH_SOCK")
    }
    pub fn configure(&self) -> Result<super::WindowsSshClientConfigurationResultWindow<'a>> {
        self.driver.press_named_enter(&self.window, "Configure")?;
        Ok(super::WindowsSshClientConfigurationResultWindow {
            driver: self.driver,
            window: self
                .driver
                .window_named(self.pid, "SSH_AUTH_SOCK configured")?,
            pid: self.pid,
        })
    }
    pub fn capture_screenshot(&self, name: &str) -> anyhow::Result<()> {
        keyringd_testkit::record_image(name, self.screenshot()?)
    }
    pub fn screenshot(&self) -> Result<std::path::PathBuf> {
        self.driver.capture(&self.window)
    }
}
