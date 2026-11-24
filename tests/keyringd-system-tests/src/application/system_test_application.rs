#[cfg(unix)]
use crate::application::features::pin_prompt_window::PinPromptWindowDriver;
use crate::application::features::{
    about::AboutWindowDriver,
    profile_setup::ProfileSetupWindowDriver,
    profiles::ProfilesWindowDriver,
    qr_prompt_window::QrPromptWindowDriver,
    security_key_window::SecurityKeyWindowDriver,
    ssh_connection_test::{
        ConnectionTestResultWindowDriver, SshConnectionRequest, SshConnectionTestWindowDriver,
    },
};
use anyhow::Result;
use keyringd_testkit::agent::AgentIdentity;
use std::path::PathBuf;

/// Owns the launched product and lends its UI feature operations.
pub trait SystemTestApplication: Sized {
    type ProfilesWindow<'a>: ProfilesWindowDriver
    where
        Self: 'a;
    type ProfileSetupWindow<'a>: ProfileSetupWindowDriver
    where
        Self: 'a;
    type AboutWindow<'a>: AboutWindowDriver
    where
        Self: 'a;
    type SshConnectionTestWindow<'a>: SshConnectionTestWindowDriver
    where
        Self: 'a;
    type ConnectionTestResultWindow<'a>: ConnectionTestResultWindowDriver
    where
        Self: 'a;
    type QrPromptWindow<'a>: QrPromptWindowDriver
    where
        Self: 'a;
    type SecurityKeyWindow<'a>: SecurityKeyWindowDriver
    where
        Self: 'a;
    #[cfg(unix)]
    type PinPromptWindow<'a>: PinPromptWindowDriver
    where
        Self: 'a;

    fn send_hybrid_advertisement(
        &self,
        advertisement: &keyringd_test_usbip::authenticator_server::client::HybridAdvertisement,
    ) -> Result<()>;
    fn start(&mut self, configuration: super::ApplicationConfiguration) -> Result<()>;
    /// Whether the application uses the real FIDO backend rather than TestPasskeyService.
    fn uses_native_fido(&self) -> Result<bool>;
    fn stop(&mut self) -> Result<()>;
    fn restart(&mut self) -> Result<()>;
    /// Clear persisted profiles and restart; navigation belongs to the caller.
    fn reset_profiles(&mut self) -> Result<()>;
    fn set_profile_save_failure(&self, fail: bool) -> Result<()>;
    fn set_ssh_client_configuration(
        &self,
        host: &str,
        configuration: &crate::ssh::SshClientConfiguration,
    ) -> Result<()>;
    fn local_timestamp(&self) -> Result<String>;
    fn agent_endpoint(&self) -> Result<String>;
    fn agent_identities(&self) -> Result<Vec<AgentIdentity>>;
    fn stored_profiles(&self) -> Result<Vec<serde_json::Value>>;
    fn connection_requests(&self) -> Result<Option<Vec<SshConnectionRequest>>>;
    /// Captures and attaches the image to the current scenario step.
    fn capture_screenshot(&self, name: &str) -> Result<()> {
        keyringd_testkit::record_image(name, self.screenshot()?)
    }
    /// Returns an image file for operations that report several observations together.
    fn screenshot(&self) -> Result<PathBuf>;
    fn drag_supported() -> Result<bool>;
    #[cfg(windows)]
    fn wait_for_recovery_idle(&self) -> Result<()>;
    #[cfg(windows)]
    fn wait_for_fido_operations_to_finish(&self) -> Result<()>;
    fn profiles_window(&mut self) -> Result<Self::ProfilesWindow<'_>>;
    fn profile_setup_window(&mut self) -> Self::ProfileSetupWindow<'_>;
    fn about_window(&mut self) -> Self::AboutWindow<'_>;
    fn ssh_connection_test_window(&mut self) -> Self::SshConnectionTestWindow<'_>;
    fn connection_test_result_window(&mut self) -> Self::ConnectionTestResultWindow<'_>;
    fn qr_prompt_window(&mut self) -> Result<Self::QrPromptWindow<'_>>;
    fn security_key_window(&mut self) -> Result<Self::SecurityKeyWindow<'_>>;
    #[cfg(unix)]
    fn pin_prompt_window(&mut self) -> Result<Self::PinPromptWindow<'_>>;
}
