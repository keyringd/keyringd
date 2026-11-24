use crate::authenticator::windows::security_automation::SecurityAutomationAction;
use crate::authenticator::windows::security_automation::SecurityAutomationClient;
use anyhow::Result;

/// Resolves the target profile's Windows Security dialog in the elevated broker.
pub struct WindowsDeviceKeySecurityWindow {
    client: SecurityAutomationClient,
    profile_name: String,
    window_handle: usize,
}
impl WindowsDeviceKeySecurityWindow {
    pub fn wait_open(profile_name: &str) -> Result<Self> {
        let client = SecurityAutomationClient::from_environment()?;
        let window_handle =
            serde_json::from_value(client.call(profile_name, SecurityAutomationAction::WaitOpen)?)?;
        Ok(Self {
            client,
            profile_name: profile_name.to_owned(),
            window_handle,
        })
    }
    pub fn capture_screenshot(&self, name: &str) -> anyhow::Result<()> {
        keyringd_testkit::record_image(name, self.screenshot()?)
    }
    pub fn screenshot(&self) -> Result<std::path::PathBuf> {
        Ok(serde_json::from_value(self.client.call(
            &self.profile_name,
            SecurityAutomationAction::Capture,
        )?)?)
    }
    pub fn create_password(&self, password: &str) -> Result<()> {
        self.client.call(
            &self.profile_name,
            SecurityAutomationAction::CreatePassword {
                value: password.to_owned(),
            },
        )?;
        Ok(())
    }
    pub fn enter_password(&self, password: &str) -> Result<()> {
        self.client.call(
            &self.profile_name,
            SecurityAutomationAction::EnterPassword {
                value: password.to_owned(),
            },
        )?;
        Ok(())
    }
    pub fn wait_for_password_creation(&self) -> Result<()> {
        self.client.call(
            &self.profile_name,
            SecurityAutomationAction::WaitEditors { count: 2 },
        )?;
        Ok(())
    }
    pub fn wait_for_password_rejection(&mut self) -> Result<()> {
        self.window_handle = serde_json::from_value(self.client.call(
            &self.profile_name,
            SecurityAutomationAction::WaitPasswordRejected,
        )?)?;
        Ok(())
    }
    pub fn wait_closed(&self) -> Result<()> {
        self.client.call(
            &self.profile_name,
            SecurityAutomationAction::WaitClosed {
                window_handle: self.window_handle,
            },
        )?;
        Ok(())
    }
}
impl Drop for WindowsDeviceKeySecurityWindow {
    fn drop(&mut self) {
        let _ = self
            .client
            .call(&self.profile_name, SecurityAutomationAction::Dismiss);
    }
}
