use crate::application::platform::macos::UiDriver;
use anyhow::Result;
use std::path::PathBuf;
pub struct MacosLicenseWindow<'a> {
    driver: UiDriver<'a>,
}
impl<'a> MacosLicenseWindow<'a> {
    pub(super) fn wait_for(driver: &UiDriver<'a>) -> Result<Self> {
        let page = Self {
            driver: driver.in_sheet("about.name"),
        };
        page.driver.wait_for("about.license-text")?;
        Ok(page)
    }
    fn read_text(&self) -> Result<String> {
        Ok(self
            .driver
            .element("about.license-text")?
            .value
            .unwrap_or_default())
    }
    fn capture(&self) -> Result<PathBuf> {
        self.driver.capture_current_window()
    }
    fn close_window(&self) -> Result<()> {
        self.driver.press("about.license-close")?;
        self.driver.wait_absent("about.license-text")
    }
}

impl super::super::LicenseDocumentDriver for MacosLicenseWindow<'_> {
    fn text(&self) -> Result<String> {
        self.read_text()
    }
    fn screenshot(&self) -> Result<PathBuf> {
        self.capture()
    }
}
impl super::super::LicenseWindowDriver for MacosLicenseWindow<'_> {
    fn close(&self) -> Result<()> {
        self.close_window()
    }
}
