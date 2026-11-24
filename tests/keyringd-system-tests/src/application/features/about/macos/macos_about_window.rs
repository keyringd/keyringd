use crate::application::features::about::macos::MacosLicenseWindow;
use crate::application::platform::macos::UiDriver;
use anyhow::Result;
use std::path::PathBuf;
pub struct MacosAboutWindow<'a> {
    driver: UiDriver<'a>,
}
impl<'a> MacosAboutWindow<'a> {
    pub fn open_from_background_menu(&mut self) -> Result<()> {
        self.driver = Self::open_from_status_menu(&self.driver)?.driver;
        Ok(())
    }

    pub fn new(app: &'a crate::environment::macos::TestApp) -> Self {
        Self {
            driver: UiDriver::new(app).in_window("about.name"),
        }
    }

    fn open_from_main_menu(driver: &UiDriver<'a>) -> Result<Self> {
        driver.invoke_app_menu_item("main-menu.application", "main-menu.about")?;
        Self::current(driver)
    }
    fn current(driver: &UiDriver<'a>) -> Result<Self> {
        let page = Self {
            driver: driver.in_window("about.name"),
        };
        page.driver
            .wait_until("about.name", "to be visible", |element| {
                element.hidden != Some(true)
            })?;
        Ok(page)
    }
    /// The status bar menu reaches About while no window is open, which is when
    /// the application menu the other entry point lives in does not exist.
    fn open_from_status_menu(driver: &UiDriver<'a>) -> Result<Self> {
        driver.invoke_status_menu_item("status-menu.about")?;
        let page = Self {
            driver: driver.in_window("about.name"),
        };
        page.driver
            .wait_until("about.name", "to be visible", |element| {
                element.hidden != Some(true)
            })?;
        Ok(page)
    }
    fn read_application_info(
        &self,
    ) -> Result<crate::application::features::about::AboutApplicationInfo> {
        let mut texts = Vec::new();
        for id in [
            "about.name",
            "about.version",
            "about.developers",
            "about.license-name",
        ] {
            let element = self.driver.element(id)?;
            if element.hidden != Some(true) {
                texts.extend(element.value);
            }
        }
        let mut enabled_actions = Vec::new();
        for id in [
            "about.website",
            "about.issues",
            "about.license",
            "about.third-party",
        ] {
            let element = self.driver.element(id)?;
            if element.enabled == Some(true) && element.hidden != Some(true) {
                enabled_actions.push(id.to_owned());
            }
        }
        Ok(crate::application::features::about::AboutApplicationInfo {
            texts,
            enabled_actions,
        })
    }
    fn open_document(&self, third_party: bool) -> Result<MacosLicenseWindow<'a>> {
        self.driver.press(if third_party {
            "about.third-party"
        } else {
            "about.license"
        })?;
        MacosLicenseWindow::wait_for(&self.driver)
    }
    fn capture(&self) -> Result<PathBuf> {
        self.driver.capture("about.name")
    }
    fn close_window(&self) -> Result<()> {
        self.driver.press("about.close")?;
        self.driver.wait_absent("about.name")
    }
}

impl super::super::AboutWindowDriver for MacosAboutWindow<'_> {
    type ApplicationLicense<'a>
        = MacosLicenseWindow<'a>
    where
        Self: 'a;
    type ThirdPartyLicensesWindow<'a>
        = MacosLicenseWindow<'a>
    where
        Self: 'a;
    fn open(&mut self) -> Result<()> {
        self.driver = Self::open_from_main_menu(&self.driver)?.driver;
        Ok(())
    }
    fn screenshot(&self) -> Result<std::path::PathBuf> {
        self.capture()
    }
    fn application_info(&self) -> Result<super::super::AboutApplicationInfo> {
        self.read_application_info()
    }
    fn open_application_license(&mut self) -> Result<Self::ApplicationLicense<'_>> {
        self.open_document(false)
    }
    fn open_third_party_licenses(&mut self) -> Result<Self::ThirdPartyLicensesWindow<'_>> {
        self.open_document(true)
    }
    fn close(&mut self) -> Result<()> {
        self.close_window()
    }
}
