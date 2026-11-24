use crate::application::platform::macos::UiDriver;
use anyhow::Result;
pub struct ReplaceDeviceKeyWindow<'a> {
    driver: UiDriver<'a>,
}
impl<'a> ReplaceDeviceKeyWindow<'a> {
    pub(super) fn wait_for_confirmation(driver: &UiDriver<'a>) -> Result<Self> {
        let driver = driver.in_sheet("profile-details.heading");
        driver.wait_for("replace-device-key.confirm")?;
        Ok(Self { driver })
    }
    pub fn result(
        &self,
    ) -> Result<crate::application::features::profiles::DeviceKeyReplacementResult> {
        let messages = self
            .driver
            .elements()?
            .into_iter()
            .filter(|element| {
                element.role.as_deref() == Some("AXStaticText") && element.hidden != Some(true)
            })
            .filter_map(|element| element.value.or(element.title))
            .collect();
        let public_keys = self.driver.element("replace-device-key.trust-entries")?;
        let close = self.driver.element("replace-device-key.close")?;
        Ok(
            crate::application::features::profiles::DeviceKeyReplacementResult {
                messages,
                public_keys_enabled: public_keys.enabled == Some(true)
                    && public_keys.hidden != Some(true),
                close_enabled: close.enabled == Some(true) && close.hidden != Some(true),
            },
        )
    }
    pub fn open_trust_entries(
        &self,
        profile_name: &str,
    ) -> Result<crate::application::features::profiles::macos::TrustEntriesWindow<'a>> {
        self.driver.wait_until(
            "replace-device-key.trust-entries",
            "to offer Public Keys",
            |element| {
                element.enabled == Some(true) && element.title.as_deref() == Some("Public Keys…")
            },
        )?;
        self.driver.press("replace-device-key.trust-entries")?;
        let trust_entries =
            crate::application::features::profiles::macos::TrustEntriesWindow::wait_for(
                &self.driver,
                profile_name,
            )?;
        self.driver.wait_absent("replace-device-key.close")?;
        self.driver
            .wait_absent("replace-device-key.trust-entries")?;
        Ok(trust_entries)
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
        self.driver.press("replace-device-key.confirm")
    }
    fn press_use_security_key(&self) -> Result<()> {
        use crate::application::features::qr_prompt_window::{
            QrPromptWindowDriver, macos::MacosQrPromptWindow,
        };
        MacosQrPromptWindow::new(&self.driver).press_use_security_key()
    }
    fn result_visible(&self) -> Result<bool> {
        Ok(self
            .driver
            .find_element("replace-device-key.trust-entries")?
            .is_some_and(|element| element.hidden != Some(true)))
    }
    fn result(&self) -> Result<crate::application::features::profiles::DeviceKeyReplacementResult> {
        ReplaceDeviceKeyWindow::result(self)
    }
    fn screenshot(&self) -> Result<std::path::PathBuf> {
        self.driver.capture_current_window()
    }
    fn open_trust_entries(&mut self, name: &str) -> Result<Self::TrustEntriesWindow<'_>> {
        ReplaceDeviceKeyWindow::open_trust_entries(self, name)
    }
}
