use crate::application::features::profiles::ProfileDetails;
use crate::application::platform::windows::UiDriver;
use anyhow::Result;
use windows::Win32::UI::Accessibility::IUIAutomationElement;

pub struct ProfileDetailsWindow<'a> {
    pub(super) driver: &'a UiDriver,
    pub(super) window: IUIAutomationElement,
    pub(super) pid: u32,
    pub(super) profile_name: String,
}

impl<'a> ProfileDetailsWindow<'a> {
    pub fn open_replacement(
        &self,
    ) -> Result<crate::application::features::profiles::windows::ReplaceDeviceKeyWindow<'a>> {
        self.driver.invoke_named(&self.window, "Replace…")?;
        self.driver.wait_window_enabled(&self.window, false)?;
        Ok(
            crate::application::features::profiles::windows::ReplaceDeviceKeyWindow {
                driver: self.driver,
                window: self.driver.window_named(self.pid, "Replace Device Key")?,
                owner: self.window.clone(),
            },
        )
    }
    pub fn details(&self) -> Result<ProfileDetails> {
        Ok(ProfileDetails {
            status: self.driver.detail_value(&self.window, "Status")?,
            signature_algorithm: self
                .driver
                .detail_value(&self.window, "SSH authentication signature format")?,
            protection: self
                .driver
                .detail_value(&self.window, "Local authentication")?,
        })
    }

    pub fn capture_screenshot(&self, name: &str) -> anyhow::Result<()> {
        keyringd_testkit::record_image(name, self.screenshot()?)
    }
    pub fn screenshot(&self) -> Result<std::path::PathBuf> {
        self.driver.capture(&self.window)
    }

    pub fn close(&self) -> Result<()> {
        self.driver.invoke_named(&self.window, "Close")?;
        self.driver
            .wait_window_enabled(&self.driver.window_named(self.pid, "Keyringd")?, true)
    }
}

impl crate::application::features::profiles::ProfileDetailsWindowDriver
    for ProfileDetailsWindow<'_>
{
    type TrustEntriesWindow<'a>
        = super::TrustEntriesWindow<'a>
    where
        Self: 'a;
    type ReplaceDeviceKeyWindow<'a>
        = super::ReplaceDeviceKeyWindow<'a>
    where
        Self: 'a;
    fn content(&self) -> Result<ProfileDetails> {
        self.details()
    }
    fn screenshot(&self) -> Result<std::path::PathBuf> {
        ProfileDetailsWindow::screenshot(self)
    }
    fn close(&mut self) -> Result<()> {
        ProfileDetailsWindow::close(self)
    }
    fn open_replacement(&mut self) -> Result<Self::ReplaceDeviceKeyWindow<'_>> {
        ProfileDetailsWindow::open_replacement(self)
    }
    fn open_trust_entries(&mut self) -> Result<Self::TrustEntriesWindow<'_>> {
        self.driver.invoke_named(&self.window, "Public Keys…")?;
        self.driver.wait_window_enabled(&self.window, false)?;
        Ok(super::TrustEntriesWindow {
            driver: self.driver,
            window: self.driver.window_named(
                self.pid,
                &format!("Public Keys for “{}”", self.profile_name),
            )?,
        })
    }
}
