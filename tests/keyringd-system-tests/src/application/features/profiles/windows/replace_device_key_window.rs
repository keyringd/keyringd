use crate::application::platform::windows::UiDriver;
use anyhow::Result;
use windows::Win32::UI::Accessibility::IUIAutomationElement;
pub struct ReplaceDeviceKeyWindow<'a> {
    pub(super) driver: &'a UiDriver,
    pub(super) window: IUIAutomationElement,
    pub(super) owner: IUIAutomationElement,
}
impl ReplaceDeviceKeyWindow<'_> {
    pub fn replace(&self) -> Result<()> {
        self.driver.invoke_named(&self.window, "Replace")
    }
    pub fn result(
        &self,
    ) -> Result<crate::application::features::profiles::DeviceKeyReplacementResult> {
        self.driver
            .wait_enabled_named(&self.window, "Public Keys…")?;
        Ok(
            crate::application::features::profiles::DeviceKeyReplacementResult {
                messages: self.driver.visible_texts(&self.window)?,
                public_keys_enabled: unsafe {
                    self.driver
                        .element_named(&self.window, "Public Keys…")?
                        .CurrentIsEnabled()?
                        .as_bool()
                },
                close_enabled: unsafe {
                    self.driver
                        .element_named(&self.window, "Close")?
                        .CurrentIsEnabled()?
                        .as_bool()
                },
            },
        )
    }
    pub fn capture_screenshot(&self, name: &str) -> anyhow::Result<()> {
        keyringd_testkit::record_image(name, self.screenshot()?)
    }
    pub fn screenshot(&self) -> Result<std::path::PathBuf> {
        self.driver.capture(&self.window)
    }
    pub fn open_trust_entries(&self, name: &str) -> Result<super::TrustEntriesWindow<'_>> {
        let pid = unsafe { self.window.CurrentProcessId()? } as u32;
        self.driver.invoke_named(&self.window, "Public Keys…")?;
        let window = self
            .driver
            .window_named(pid, &format!("Public Keys for “{name}”"))?;
        self.driver.wait_window_absent(pid, "Replace Device Key")?;
        self.driver.wait_window_enabled(&self.owner, false)?;
        self.driver
            .wait_visible_named(&window, "Device Key Public Key")?;
        self.driver
            .wait_visible_named(&window, "Certificate Authority Public Key")?;
        Ok(super::TrustEntriesWindow {
            driver: self.driver,
            window,
        })
    }
}
impl crate::application::features::profiles::ReplaceDeviceKeyWindowDriver
    for ReplaceDeviceKeyWindow<'_>
{
    type TrustEntriesWindow<'a>
        = super::TrustEntriesWindow<'a>
    where
        Self: 'a;
    fn confirm(&self) -> Result<()> {
        self.replace()
    }
    fn press_use_security_key(&self) -> Result<()> {
        let pid = unsafe { self.window.CurrentProcessId()? } as u32;
        let window = self
            .driver
            .window_named(pid, "Authorize the New Device Key")?;
        self.driver.invoke_named(&window, "Use security key…")
    }
    fn result_visible(&self) -> Result<bool> {
        self.driver
            .contains_visible_named(&self.window, "Public Keys…")
    }
    fn result(&self) -> Result<crate::application::features::profiles::DeviceKeyReplacementResult> {
        ReplaceDeviceKeyWindow::result(self)
    }
    fn screenshot(&self) -> Result<std::path::PathBuf> {
        ReplaceDeviceKeyWindow::screenshot(self)
    }
    fn open_trust_entries(&mut self, name: &str) -> Result<Self::TrustEntriesWindow<'_>> {
        ReplaceDeviceKeyWindow::open_trust_entries(self, name)
    }
}
