use crate::application::features::profiles::windows::WindowsProfilesWindow;
use crate::application::platform::windows::UiDriver;
use anyhow::Result;
pub struct UiApplication<'a> {
    driver: &'a UiDriver,
    pid: u32,
}

impl<'a> UiApplication<'a> {
    pub fn open_main_menu(
        &self,
    ) -> Result<crate::application::features::navigation::windows::ApplicationMenuPage<'a>> {
        self.driver.open_tray_menu(self.pid)?;
        Ok(
            crate::application::features::navigation::windows::ApplicationMenuPage {
                driver: self.driver,
                pid: self.pid,
            },
        )
    }
    pub fn new(driver: &'a UiDriver, pid: u32) -> Self {
        Self { driver, pid }
    }

    pub fn public_keys(
        &self,
        name: &str,
    ) -> Result<crate::application::features::profiles::windows::TrustEntriesWindow<'a>> {
        Ok(
            crate::application::features::profiles::windows::TrustEntriesWindow {
                driver: self.driver,
                window: self
                    .driver
                    .window_named(self.pid, &format!("Public Keys for “{name}”"))?,
            },
        )
    }

    pub fn qr_prompt_window(
        &self,
    ) -> crate::application::features::qr_prompt_window::windows::WindowsQrPromptWindow<'a> {
        crate::application::features::qr_prompt_window::windows::WindowsQrPromptWindow::new(
            self.driver,
            self.pid,
        )
    }

    pub fn profiles_window(&self) -> Result<WindowsProfilesWindow<'a>> {
        Ok(WindowsProfilesWindow {
            driver: self.driver,
            window: self.driver.window_named(self.pid, "Keyringd")?,
            pid: self.pid,
        })
    }

    pub fn capture_screenshot(&self, name: &str) -> anyhow::Result<()> {
        keyringd_testkit::record_image(name, self.screenshot()?)
    }
    pub fn screenshot(&self) -> Result<std::path::PathBuf> {
        self.driver.wait("application screenshot", || {
            for title in [
                "Profile Details",
                "Remove SSH profile",
                "Add SSH profile",
                "Keyringd",
            ] {
                if let Some(window) = self.driver.find_window_named(self.pid, title)? {
                    return Ok(Some(self.driver.capture(&window)?));
                }
            }
            Ok(None)
        })
    }
}
