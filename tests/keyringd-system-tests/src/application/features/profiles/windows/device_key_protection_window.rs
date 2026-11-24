use crate::application::platform::windows::UiDriver;
use anyhow::Result;
use windows::Win32::UI::Accessibility::IUIAutomationElement;

/// Product confirmation dialog; Windows Security is a DeviceKeyService boundary.
pub struct DeviceKeyProtectionWindow<'a> {
    pub(super) driver: &'a UiDriver,
    pub(super) window: IUIAutomationElement,
    pub(super) owner: IUIAutomationElement,
}

impl DeviceKeyProtectionWindow<'_> {
    pub fn continue_change(&self) -> Result<()> {
        self.driver.invoke_named(&self.window, "Continue")
    }

    pub fn cancel(&self) -> Result<()> {
        self.driver.invoke_named(&self.window, "Cancel")?;
        self.driver.wait_window_enabled(&self.owner, true)
    }

    pub fn capture_screenshot(&self, name: &str) -> anyhow::Result<()> {
        keyringd_testkit::record_image(name, self.screenshot()?)
    }
    pub fn screenshot(&self) -> Result<std::path::PathBuf> {
        self.driver.capture(&self.window)
    }
}
