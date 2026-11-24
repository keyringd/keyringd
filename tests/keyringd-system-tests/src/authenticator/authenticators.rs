use super::{HybridPhone, UsbSecurityKey};
use anyhow::{Context, Result};
use keyringd_test_usbip::authenticator_server::client::{
    AuthenticatorConfiguration, AuthenticatorHandle, AuthenticatorServerClient,
};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::runtime::Runtime;

/// Resource factory for one environment; every device uses the same runtime.
pub struct Authenticators {
    runtime: Arc<Runtime>,
    server: AuthenticatorServerClient,
}
impl Authenticators {
    pub fn usb_available() -> bool {
        #[cfg(target_os = "linux")]
        {
            true
        }
        #[cfg(target_os = "macos")]
        {
            [
                "KEYRINGD_TEST_AUTHENTICATOR_CONTROL",
                "KEYRINGD_TEST_USBIP_SERVER",
            ]
            .into_iter()
            .all(|name| std::env::var(name).is_ok_and(|value| !value.is_empty()))
        }
        #[cfg(target_os = "windows")]
        {
            std::env::var("KEYRINGD_TEST_AUTHENTICATOR_CONTROL")
                .is_ok_and(|value| !value.is_empty())
                && unsafe {
                    windows::Win32::UI::WindowsAndMessaging::GetSystemMetrics(
                        windows::Win32::UI::WindowsAndMessaging::SM_REMOTESESSION,
                    ) == 0
                }
        }
    }
    pub fn hybrid_available() -> bool {
        #[cfg(target_os = "linux")]
        {
            true
        }
        #[cfg(not(target_os = "linux"))]
        {
            [
                "KEYRINGD_TEST_AUTHENTICATOR_CONTROL",
                "KEYRINGD_UI_TEST_HYBRID_TUNNEL_ADDRESS",
                "KEYRINGD_UI_TEST_HYBRID_TUNNEL_CA",
            ]
            .into_iter()
            .all(|name| std::env::var(name).is_ok_and(|value| !value.is_empty()))
        }
    }
    pub fn from_environment(runtime: Arc<Runtime>) -> Result<Self> {
        #[cfg(target_os = "linux")]
        let variable = "KEYRINGD_E2E_AUTHENTICATOR_CONTROL";
        #[cfg(not(target_os = "linux"))]
        let variable = "KEYRINGD_TEST_AUTHENTICATOR_CONTROL";
        let endpoint =
            std::env::var(variable).with_context(|| format!("{variable} is required"))?;
        Self::connect(runtime, &endpoint)
    }
    pub fn connect(runtime: Arc<Runtime>, endpoint: &str) -> Result<Self> {
        let server = AuthenticatorServerClient::from_endpoint(endpoint)?;
        runtime.block_on(server.health())?;
        Ok(Self { runtime, server })
    }
    /// Returns an already connected, independently owned Security Key.
    pub fn create_security_key(
        &self,
        configuration: AuthenticatorConfiguration,
    ) -> Result<UsbSecurityKey> {
        let handle = self
            .runtime
            .block_on(self.server.create_authenticator(configuration))?;
        let connection = match self.usb_connection(&handle) {
            Ok(connection) => connection,
            Err(error) => {
                if let Err(cleanup) = self.runtime.block_on(handle.delete()) {
                    return Err(
                        error.context(format!("also failed to delete partial key: {cleanup}"))
                    );
                }
                return Err(error);
            }
        };
        let key = UsbSecurityKey::new(Arc::clone(&self.runtime), handle, connection);
        key.connect()?;
        Ok(key)
    }
    pub fn create_hybrid_phone(
        &self,
        configuration: AuthenticatorConfiguration,
    ) -> Result<HybridPhone> {
        let handle = self
            .runtime
            .block_on(self.server.create_authenticator(configuration))?;
        if let Err(error) = self.runtime.block_on(handle.detach()) {
            let cleanup = self.runtime.block_on(handle.delete());
            return Err(anyhow::Error::from(error).context(format!(
                "could not remove phone USB export; deletion: {cleanup:?}"
            )));
        }
        let phone = HybridPhone::new(Arc::clone(&self.runtime), handle);
        Ok(phone)
    }
    fn usb_connection(
        &self,
        handle: &AuthenticatorHandle,
    ) -> Result<Option<Box<dyn super::UsbConnection>>> {
        #[cfg(target_os = "linux")]
        {
            Ok(Some(Box::new(
                super::linux::LinuxUsbConnection::from_environment(handle.id())?,
            )))
        }
        #[cfg(target_os = "macos")]
        {
            let _ = handle;
            Ok(None)
        }
        #[cfg(target_os = "windows")]
        {
            Ok(Some(Box::new(super::windows::WindowsUsbConnection::new(
                self.usb_address()?,
                self.runtime.block_on(handle.info())?.bus_id,
            )?)))
        }
    }
    pub fn usb_address(&self) -> Result<SocketAddr> {
        std::env::var("KEYRINGD_TEST_USBIP_SERVER")
            .context("KEYRINGD_TEST_USBIP_SERVER is required")?
            .parse()
            .map_err(Into::into)
    }
}
