use crate::application::platform::windows::UiDriver;
use anyhow::Result;
use windows::Win32::UI::Accessibility::IUIAutomationElement;
pub struct WindowsSshClientConfigurationResultWindow<'a> {
    pub(super) driver: &'a UiDriver,
    pub(super) window: IUIAutomationElement,
    pub(super) pid: u32,
}
impl WindowsSshClientConfigurationResultWindow<'_> {
    pub fn content(&self) -> Result<Vec<String>> {
        self.driver.visible_texts(&self.window)
    }
    pub fn screenshot(&self) -> Result<std::path::PathBuf> {
        self.driver.capture(&self.window)
    }
    pub fn capture_screenshot(&self, name: &str) -> Result<()> {
        keyringd_testkit::record_image(name, self.screenshot()?)
    }
    pub fn close(&self) -> Result<()> {
        self.driver.invoke_named(&self.window, "OK")?;
        self.driver
            .wait_window_absent(self.pid, "SSH_AUTH_SOCK configured")
    }
}
