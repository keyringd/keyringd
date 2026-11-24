use crate::application::platform::windows::UiDriver;
use anyhow::Result;
use windows::Win32::UI::Accessibility::IUIAutomationElement;

pub struct WindowsGitSshClientConfigurationWindow<'a> {
    pub(super) driver: &'a UiDriver,
    pub(super) window: IUIAutomationElement,
    pub(super) pid: u32,
}

impl WindowsGitSshClientConfigurationWindow<'_> {
    pub fn copy(&self) -> Result<String> {
        let sequence = self.driver.clipboard_sequence();
        self.driver
            .press_named_enter(&self.window, "Copy SSH setting")?;
        self.driver
            .wait_window_absent(self.pid, "Use Keyringd with Git for Windows")?;
        self.driver.clipboard_after(sequence)
    }

    pub fn cancel(&self) -> Result<()> {
        self.driver.invoke_named(&self.window, "Cancel")?;
        self.driver
            .wait_window_absent(self.pid, "Use Keyringd with Git for Windows")
    }

    pub fn capture_screenshot(&self, name: &str) -> anyhow::Result<()> {
        keyringd_testkit::record_image(name, self.screenshot()?)
    }
    pub fn screenshot(&self) -> Result<std::path::PathBuf> {
        self.driver.capture(&self.window)
    }
}
