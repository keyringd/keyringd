use crate::application::features::navigation::windows::WindowsSshClientConfigurationConfirmationWindow;
use crate::application::platform::windows::UiDriver;
use anyhow::Result;

pub struct WindowsSshClientConfigurationWindow<'a> {
    pub(super) driver: &'a UiDriver,
    pub(super) pid: u32,
}
impl<'a> WindowsSshClientConfigurationWindow<'a> {
    pub(crate) fn current(driver: &'a UiDriver, pid: u32) -> Result<Self> {
        driver.window_named(pid, "Configure SSH Clients to Use Keyringd")?;
        Ok(Self { driver, pid })
    }
    pub fn open_git_for_windows_setting(
        &self,
    ) -> Result<
        crate::application::features::navigation::windows::WindowsGitSshClientConfigurationWindow<
            'a,
        >,
    > {
        self.driver.press_named_enter(
            &self
                .driver
                .window_named(self.pid, "Configure SSH Clients to Use Keyringd")?,
            "Copy Git for Windows SSH setting",
        )?;
        Ok(
            crate::application::features::navigation::windows::WindowsGitSshClientConfigurationWindow {
                driver: self.driver,
                window: self
                    .driver
                    .window_named(self.pid, "Use Keyringd with Git for Windows")?,
                pid: self.pid,
            },
        )
    }
    pub fn open_windows_configuration(
        &self,
    ) -> Result<WindowsSshClientConfigurationConfirmationWindow<'a>> {
        self.driver.press_named_enter(
            &self
                .driver
                .window_named(self.pid, "Configure SSH Clients to Use Keyringd")?,
            "SSH_AUTH_SOCK: Configure",
        )?;
        Ok(WindowsSshClientConfigurationConfirmationWindow {
            driver: self.driver,
            window: self.driver.window_named(self.pid, "SSH_AUTH_SOCK")?,
            pid: self.pid,
        })
    }
    pub fn unconfigure(&self) -> Result<()> {
        self.driver.press_named_enter(
            &self
                .driver
                .window_named(self.pid, "Configure SSH Clients to Use Keyringd")?,
            "SSH_AUTH_SOCK: Unconfigure",
        )?;
        let confirmation = self.driver.window_named(self.pid, "SSH_AUTH_SOCK")?;
        self.driver
            .press_named_enter(&confirmation, "Unconfigure")?;
        let result = self
            .driver
            .window_named(self.pid, "SSH_AUTH_SOCK unconfigured")?;
        self.driver.invoke_named(&result, "OK")?;
        self.driver
            .wait_window_absent(self.pid, "SSH_AUTH_SOCK unconfigured")?;
        Ok(())
    }
    pub fn capture_screenshot(&self, name: &str) -> anyhow::Result<()> {
        keyringd_testkit::record_image(name, self.screenshot()?)
    }
    pub fn screenshot(&self) -> Result<std::path::PathBuf> {
        self.driver.capture(
            &self
                .driver
                .window_named(self.pid, "Configure SSH Clients to Use Keyringd")?,
        )
    }
    pub fn close(&self) -> Result<()> {
        self.driver.invoke_named(
            &self
                .driver
                .window_named(self.pid, "Configure SSH Clients to Use Keyringd")?,
            "Close",
        )?;
        self.driver
            .wait_window_absent(self.pid, "Configure SSH Clients to Use Keyringd")
    }
}
