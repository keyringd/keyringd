use std::path::PathBuf;

use anyhow::Result;

use crate::application::features::profiles::ProfileDetails;
use crate::application::platform::macos::UiDriver;

const HEADING: &str = "profile-details.heading";
const STATUS: &str = "profile-details.status";
const SSH_FORMAT: &str = "profile-details.device-ssh-format";
const DEVICE_KEY_PROTECTION: &str = "profile-details.local-authentication";
const CLOSE: &str = "profile-details.close";

/// The product Profile Details window.
pub struct ProfileDetailsWindow<'a> {
    driver: UiDriver<'a>,
    profile_name: String,
}

impl<'a> ProfileDetailsWindow<'a> {
    pub fn wait_for(driver: &UiDriver<'a>, profile_name: &str) -> Result<Self> {
        let page = Self {
            driver: driver.in_window(HEADING),
            profile_name: profile_name.to_owned(),
        };
        page.driver
            .wait_value(HEADING, &format!("Details for “{profile_name}”"))?;
        Ok(page)
    }

    pub fn capture_screenshot(&self, name: &str) -> anyhow::Result<()> {
        keyringd_testkit::record_image(name, self.screenshot()?)
    }
    pub fn screenshot(&self) -> Result<PathBuf> {
        self.driver.capture(HEADING)
    }

    pub fn details(&self) -> Result<ProfileDetails> {
        Ok(ProfileDetails {
            status: self
                .driver
                .wait_for(STATUS)?
                .value
                .ok_or_else(|| anyhow::anyhow!("Status has no value"))?,
            signature_algorithm: self
                .driver
                .wait_for(SSH_FORMAT)?
                .value
                .ok_or_else(|| anyhow::anyhow!("Signature format has no value"))?,
            protection: self
                .driver
                .wait_for(DEVICE_KEY_PROTECTION)?
                .value
                .ok_or_else(|| anyhow::anyhow!("Local authentication has no value"))?,
        })
    }

    pub fn close(&self) -> Result<()> {
        self.driver.press(CLOSE)?;
        self.driver.wait_absent(HEADING)
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
    fn screenshot(&self) -> Result<PathBuf> {
        ProfileDetailsWindow::screenshot(self)
    }
    fn close(&mut self) -> Result<()> {
        ProfileDetailsWindow::close(self)
    }
    fn open_trust_entries(&mut self) -> Result<Self::TrustEntriesWindow<'_>> {
        self.driver.press("profile-details.trust-entries")?;
        super::TrustEntriesWindow::wait_for(&self.driver, &self.profile_name)
    }
    fn open_replacement(&mut self) -> Result<Self::ReplaceDeviceKeyWindow<'_>> {
        self.driver.press("profile-details.replace-device-key")?;
        super::ReplaceDeviceKeyWindow::wait_for_confirmation(&self.driver)
    }
}
