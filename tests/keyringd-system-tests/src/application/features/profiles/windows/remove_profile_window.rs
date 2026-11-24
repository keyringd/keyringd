use crate::application::platform::windows::UiDriver;
use anyhow::Result;
use windows::Win32::UI::Accessibility::IUIAutomationElement;

pub struct RemoveProfileWindow<'a> {
    pub(super) driver: &'a UiDriver,
    pub(super) window: IUIAutomationElement,
    pub(super) pid: u32,
}

impl RemoveProfileWindow<'_> {
    pub fn capture_screenshot(&self, name: &str) -> anyhow::Result<()> {
        keyringd_testkit::record_image(name, self.screenshot()?)
    }
    pub fn screenshot(&self) -> Result<std::path::PathBuf> {
        self.driver.capture(&self.window)
    }

    pub fn confirm(&self) -> Result<()> {
        self.driver.invoke_named(&self.window, "Remove")?;
        self.driver
            .wait_window_enabled(&self.driver.window_named(self.pid, "Keyringd")?, true)
    }
}

impl crate::application::features::profiles::RemoveProfileWindowDriver for RemoveProfileWindow<'_> {
    fn screenshot(&self) -> Result<std::path::PathBuf> {
        RemoveProfileWindow::screenshot(self)
    }
    fn confirm(&mut self) -> Result<()> {
        RemoveProfileWindow::confirm(self)
    }
}
