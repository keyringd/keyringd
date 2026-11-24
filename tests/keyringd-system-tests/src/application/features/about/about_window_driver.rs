use super::{AboutApplicationInfo, LicenseWindowDriver};
use anyhow::Result;
use std::path::PathBuf;

pub trait AboutWindowDriver {
    type ApplicationLicense<'a>: LicenseWindowDriver
    where
        Self: 'a;
    type ThirdPartyLicensesWindow<'a>: LicenseWindowDriver
    where
        Self: 'a;
    fn open(&mut self) -> Result<()>;
    fn screenshot(&self) -> Result<PathBuf>;
    fn capture_screenshot(&self, name: &str) -> Result<()> {
        keyringd_testkit::record_image(name, self.screenshot()?)
    }
    fn application_info(&self) -> Result<AboutApplicationInfo>;
    fn open_application_license(&mut self) -> Result<Self::ApplicationLicense<'_>>;
    fn open_third_party_licenses(&mut self) -> Result<Self::ThirdPartyLicensesWindow<'_>>;
    fn close(&mut self) -> Result<()>;
}
