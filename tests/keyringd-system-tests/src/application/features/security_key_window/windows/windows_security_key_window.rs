use super::super::SecurityKeyWindowDriver;
use crate::authenticator::windows::security_automation::{
    SecurityAutomationAction, SecurityAutomationClient,
};
use anyhow::{Result, ensure};
use std::path::PathBuf;

/// The application's native Windows Security dialog, observed by the elevated broker.
pub struct WindowsSecurityKeyWindow {
    client: SecurityAutomationClient,
}
impl WindowsSecurityKeyWindow {
    pub fn new() -> Result<Self> {
        Ok(Self {
            client: SecurityAutomationClient::from_environment()?,
        })
    }
}
impl SecurityKeyWindowDriver for WindowsSecurityKeyWindow {
    fn wait_open(&self) -> Result<()> {
        self.client
            .call("", SecurityAutomationAction::WaitWebAuthnOpen)?;
        Ok(())
    }
    fn screenshot(&self) -> Result<PathBuf> {
        Ok(serde_json::from_value(
            self.client
                .call("", SecurityAutomationAction::CaptureWebAuthn)?,
        )?)
    }
    fn wait_closed(&self) -> Result<()> {
        self.client
            .call("", SecurityAutomationAction::WaitWebAuthnClosed)?;
        Ok(())
    }
    fn pin_visible(&self) -> Result<bool> {
        Ok(self
            .client
            .call("", SecurityAutomationAction::WebAuthnEditors)?
            == 1)
    }
    fn enter_pin(&self, pin: &str) -> Result<()> {
        ensure!(
            self.pin_visible()?,
            "the requested PIN editor disappeared before entry"
        );
        self.client.call(
            "",
            SecurityAutomationAction::EnterWebAuthnPin {
                value: pin.to_owned(),
            },
        )?;
        Ok(())
    }
}
