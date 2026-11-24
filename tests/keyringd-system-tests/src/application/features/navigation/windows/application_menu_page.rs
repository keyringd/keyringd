use crate::application::platform::windows::UiDriver;
use anyhow::Result;

pub struct ApplicationMenuPage<'a> {
    pub(super) driver: &'a UiDriver,
    pub(super) pid: u32,
}

impl<'a> ApplicationMenuPage<'a> {
    pub fn open_about(
        &self,
    ) -> Result<crate::application::features::about::windows::WindowsAboutWindow<'a>> {
        self.driver
            .invoke_application_named(self.pid, "About Keyringd")?;
        Ok(
            crate::application::features::about::windows::WindowsAboutWindow {
                driver: self.driver,
                pid: self.pid,
            },
        )
    }
    pub fn open_ssh_client_configuration(
        &self,
    ) -> Result<
        crate::application::features::navigation::windows::WindowsSshClientConfigurationWindow<'a>,
    > {
        self.driver
            .invoke_application_named(self.pid, "Configure SSH Clients…")?;
        Ok(
            crate::application::features::navigation::windows::WindowsSshClientConfigurationWindow {
                driver: self.driver,
                pid: self.pid,
            },
        )
    }
    pub fn exit(&self) -> Result<()> {
        self.driver.invoke_application_named(self.pid, "Exit")
    }
    pub fn capture_screenshot(&self, name: &str) -> anyhow::Result<()> {
        keyringd_testkit::record_image(name, self.screenshot()?)
    }
    pub fn screenshot(&self) -> Result<std::path::PathBuf> {
        let window = self
            .driver
            .window_with_text(self.pid, "Configure SSH Clients…")?;
        self.driver.capture(&window)
    }
}
