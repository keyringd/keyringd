use crate::application::platform::windows::UiDriver;
use anyhow::Result;
use windows::Win32::UI::Accessibility::IUIAutomationElement;

pub struct WindowsLicenseWindow<'a> {
    pub(super) driver: &'a UiDriver,
    pub(super) window: IUIAutomationElement,
    pub(super) owner: IUIAutomationElement,
}
impl WindowsLicenseWindow<'_> {
    fn read_text(&self) -> Result<String> {
        self.driver.text(&self.window, "license-text")
    }
    pub fn scroll_to_bottom(&self) -> Result<()> {
        self.driver.scroll_to_bottom(&self.window)
    }
    fn capture(&self) -> Result<std::path::PathBuf> {
        self.driver.capture(&self.window)
    }
    fn close_window(&self) -> Result<()> {
        self.driver.invoke_named(&self.window, "Close")?;
        self.driver.wait_window_enabled(&self.owner, true)
    }
}

impl super::super::LicenseDocumentDriver for WindowsLicenseWindow<'_> {
    fn text(&self) -> Result<String> {
        self.read_text()
    }
    fn screenshot(&self) -> Result<std::path::PathBuf> {
        self.capture()
    }
}
impl super::super::LicenseWindowDriver for WindowsLicenseWindow<'_> {
    fn close(&self) -> Result<()> {
        self.close_window()
    }
}
