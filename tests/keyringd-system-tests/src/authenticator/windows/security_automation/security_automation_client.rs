use super::SecurityAutomationAction;
use super::security_automation_request::SecurityAutomationRequest;
use anyhow::{Context, Result};
use serde_json::Value;
use std::io::{BufRead, BufReader, Write};
use std::net::{SocketAddr, TcpStream};
use std::time::Duration;

pub struct SecurityAutomationClient {
    address: SocketAddr,
    token: String,
}
impl SecurityAutomationClient {
    pub fn new(address: SocketAddr, token: String) -> Result<Self> {
        anyhow::ensure!(
            address.ip().is_loopback(),
            "Security Automation endpoint must be loopback"
        );
        Ok(Self { address, token })
    }
    pub fn from_environment() -> Result<Self> {
        Self::new(
            std::env::var("KEYRINGD_SECURITY_AUTOMATION_ADDRESS")
                .context("Security Automation broker is not configured")?
                .parse()?,
            std::env::var("KEYRINGD_SECURITY_AUTOMATION_TOKEN")?,
        )
    }
    pub fn call(&self, profile_name: &str, action: SecurityAutomationAction) -> Result<Value> {
        let operation = match &action {
            SecurityAutomationAction::IsElevated => "inspect elevation",
            SecurityAutomationAction::WaitOpen => "wait open",
            SecurityAutomationAction::WaitClosed { .. } => "wait closed",
            SecurityAutomationAction::Capture => "capture",
            SecurityAutomationAction::WaitEditors { .. } => "wait password fields",
            SecurityAutomationAction::CreatePassword { .. } => "create password",
            SecurityAutomationAction::EnterPassword { .. } => "enter password",
            SecurityAutomationAction::WaitPasswordRejected => "wait password rejection",
            SecurityAutomationAction::Dismiss => "dismiss",
            SecurityAutomationAction::WaitWebAuthnOpen => "wait WebAuthn open",
            SecurityAutomationAction::WaitWebAuthnClosed => "wait WebAuthn closed",
            SecurityAutomationAction::CaptureWebAuthn => "capture WebAuthn",
            SecurityAutomationAction::WebAuthnEditors => "inspect WebAuthn PIN field",
            SecurityAutomationAction::EnterWebAuthnPin { .. } => "enter WebAuthn PIN",
            SecurityAutomationAction::Shutdown => "shutdown",
        };
        let mut stream = TcpStream::connect_timeout(&self.address, Duration::from_secs(5))?;
        stream.set_read_timeout(Some(Duration::from_secs(30)))?;
        stream.set_write_timeout(Some(Duration::from_secs(5)))?;
        serde_json::to_writer(
            &mut stream,
            &SecurityAutomationRequest {
                token: self.token.clone(),
                profile_name: profile_name.to_owned(),
                action,
            },
        )?;
        stream.write_all(b"\n")?;
        let mut response = String::new();
        BufReader::new(stream)
            .read_line(&mut response)
            .with_context(|| format!("Security Automation {operation} for {profile_name:?}"))?;
        let response: std::result::Result<Value, String> = serde_json::from_str(&response)?;
        response.map_err(|error| anyhow::anyhow!("Windows Security Automation: {error}"))
    }
}
