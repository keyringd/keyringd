use super::super::ConnectionTestResultWindowDriver;
use crate::application::platform::windows::UiDriver;
use crate::environment::windows::TestApp;
use anyhow::Result;
use windows::Win32::UI::Accessibility::IUIAutomationElement;
pub struct WindowsConnectionTestResultWindow<'a> {
    driver: &'a UiDriver,
    pid: u32,
}
impl<'a> WindowsConnectionTestResultWindow<'a> {
    pub(crate) fn new(app: &TestApp, driver: &'a UiDriver) -> Self {
        Self {
            driver,
            pid: app.pid(),
        }
    }

    fn window(&self) -> Result<IUIAutomationElement> {
        let window = self
            .driver
            .window_named(self.pid, "SSH Connection Result")?;
        self.driver
            .wait_window_enabled(&self.driver.owner_window(&window)?, false)?;
        Ok(window)
    }
}
impl ConnectionTestResultWindowDriver for WindowsConnectionTestResultWindow<'_> {
    fn visible(&self) -> Result<bool> {
        Ok(
            crate::application::platform::windows::window_locator::named_window(
                self.pid,
                "SSH Connection Result",
            )?
            .is_some(),
        )
    }
    fn content(
        &self,
    ) -> Result<crate::application::features::ssh_connection_test::SshConnectionResult> {
        let window = self.window()?;
        use crate::application::features::ssh_connection_test::SshConnectionResult;
        use crate::application::features::ssh_connection_test::SshRejectionGuidance;
        let read = |id: &str| -> Result<String> {
            let element = self.driver.element(&window, id)?;
            anyhow::ensure!(
                !unsafe { element.CurrentIsOffscreen()?.as_bool() },
                "result element is hidden: {id}"
            );
            self.driver.text(&window, id)
        };
        let title = read("ssh-test-result-heading")?;
        let host_label = read("ssh-test-result-host")?;
        let host = host_label
            .strip_prefix("Host: ")
            .ok_or_else(|| anyhow::anyhow!("invalid host label: {host_label}"))?
            .to_owned();
        let diagnostic = read("ssh-test-result-body")?;
        let rejection = if !self
            .driver
            .contains_automation_id(&window, "ssh-test-rejection-entry")?
        {
            None
        } else {
            Some(SshRejectionGuidance {
                entry: read("ssh-test-rejection-entry")?,
                instructions: [
                    read("ssh-test-authorized-keys")?,
                    read("ssh-test-server-guidance")?,
                ]
                .join("\n"),
            })
        };
        Ok(SshConnectionResult {
            title,
            host,
            diagnostic,
            rejection,
        })
    }
    fn copy_rejected_entry(&self) -> Result<String> {
        let window = self.window()?;
        let sequence = self.driver.clipboard_sequence();
        self.driver.invoke(&window, "ssh-test-copy-entry")?;
        self.driver.clipboard_after(sequence)
    }
    fn screenshot(&self) -> Result<std::path::PathBuf> {
        let window = self.window()?;
        self.driver.capture(&window)
    }
    fn close(&self) -> Result<()> {
        let window = self.window()?;
        let owner = self.driver.owner_window(&window)?;
        self.driver.invoke_named(&window, "Close")?;
        self.driver.wait_window_enabled(&owner, true)
    }
}
