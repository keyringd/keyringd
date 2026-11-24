use super::super::{SshConnectionIdentity, SshConnectionTestWindowDriver};
use crate::application::features::navigation::windows::UiApplication;
use crate::application::platform::windows::UiDriver;
use crate::environment::windows::TestApp;
use anyhow::Result;
use keyringd_core::model::ProfileSshIdentityKind;
use windows::Win32::UI::Accessibility::IUIAutomationElement;
pub struct WindowsSshConnectionTestWindow<'a> {
    driver: &'a UiDriver,
    pid: u32,
}
impl<'a> WindowsSshConnectionTestWindow<'a> {
    pub(crate) fn new(app: &TestApp, driver: &'a UiDriver) -> Self {
        Self {
            driver,
            pid: app.pid(),
        }
    }
    fn window(&self) -> Result<IUIAutomationElement> {
        self.driver.window_with_id(self.pid, "ssh-test-host")
    }
}
impl SshConnectionTestWindowDriver for WindowsSshConnectionTestWindow<'_> {
    fn open(&mut self, profile: &str) -> Result<()> {
        UiApplication::new(self.driver, self.pid)
            .profiles_window()?
            .open_connection_test(profile)?;
        self.window()?;
        Ok(())
    }
    fn selected_identities(
        &self,
    ) -> Result<Vec<crate::application::features::ssh_connection_test::SshConnectionIdentity>> {
        let window = self.window()?;
        use crate::application::features::ssh_connection_test::SshConnectionIdentity;
        use windows::Win32::UI::Accessibility::{
            IUIAutomationSelectionItemPattern, UIA_SelectionItemPatternId,
        };
        let mut selected = Vec::new();
        for (index, choice) in [
            SshConnectionIdentity::DeviceCertificate,
            SshConnectionIdentity::DeviceKey,
            SshConnectionIdentity::Passkey,
        ]
        .into_iter()
        .enumerate()
        {
            let element = self
                .driver
                .element(&window, &format!("ssh-test-identity-{index}"))?;
            let pattern = unsafe {
                element.GetCurrentPatternAs::<IUIAutomationSelectionItemPattern>(
                    UIA_SelectionItemPatternId,
                )?
            };
            if unsafe { pattern.CurrentIsSelected()?.as_bool() } {
                selected.push(choice);
            }
        }
        Ok(selected)
    }
    fn select_identity(&self, identity: SshConnectionIdentity) -> Result<()> {
        let window = self.window()?;
        let kind = native_identity(identity);
        let index = ProfileSshIdentityKind::ALL
            .iter()
            .position(|value| *value == kind)
            .unwrap();
        self.driver
            .select_item(&window, &format!("ssh-test-identity-{index}"))
    }
    fn start(&self, host: &str) -> Result<()> {
        let window = self.window()?;
        self.driver.set_value(&window, "ssh-test-host", host)?;
        self.driver.wait_enabled(&window, "ssh-test-start", true)?;
        self.driver.invoke(&window, "ssh-test-start")
    }
    fn screenshot(&self) -> Result<std::path::PathBuf> {
        let window = self.window()?;
        self.driver.capture(&window)
    }
    fn close(&mut self) -> Result<()> {
        self.driver.invoke_named(&self.window()?, "Close")?;
        self.driver
            .wait_window_enabled(&self.driver.window_named(self.pid, "Keyringd")?, true)?;
        Ok(())
    }
}
fn native_identity(identity: SshConnectionIdentity) -> ProfileSshIdentityKind {
    match identity {
        SshConnectionIdentity::DeviceCertificate => ProfileSshIdentityKind::DeviceCertificate,
        SshConnectionIdentity::DeviceKey => ProfileSshIdentityKind::DeviceKey,
        SshConnectionIdentity::Passkey => ProfileSshIdentityKind::Passkey,
    }
}
