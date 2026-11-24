use crate::application::features::pin_prompt_window::macos::MacosPinPromptWindow;
use crate::application::features::ssh_connection_test::SshConnectionRequest;
use crate::application::features::{
    about::macos::MacosAboutWindow,
    profile_setup::macos::{MacosProfileSetupWindow, MacosProfileSetupWizard},
    profiles::macos::MacosProfilesWindow,
    qr_prompt_window::macos::MacosQrPromptWindow,
    security_key_window::macos::MacosSecurityKeyWindow,
    ssh_connection_test::macos::{MacosConnectionTestResultWindow, MacosSshConnectionTestWindow},
};

use crate::application::platform::macos::UiDriver;
use crate::environment::macos::TestApp;
use anyhow::{Context, Result};
use keyringd_testkit::agent::AgentIdentity;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use crate::application::SystemTestApplication;

pub struct MacosSystemTestApplication {
    app: Option<TestApp>,
    runtime: std::sync::Arc<tokio::runtime::Runtime>,
    authenticator_usb_endpoint: std::net::SocketAddr,
}
impl MacosSystemTestApplication {
    #[cfg(target_os = "macos")]
    pub(crate) fn new(
        runtime: std::sync::Arc<tokio::runtime::Runtime>,
        authenticator_usb_endpoint: std::net::SocketAddr,
    ) -> Result<Self> {
        Ok(Self {
            app: None,
            runtime,
            authenticator_usb_endpoint,
        })
    }
    fn app(&self) -> &TestApp {
        self.app
            .as_ref()
            .expect("application must be started before its features are used")
    }
    pub fn agent_socket(&self) -> Result<String> {
        self.app().agent_socket()
    }
}
impl SystemTestApplication for MacosSystemTestApplication {
    fn send_hybrid_advertisement(
        &self,
        advertisement: &keyringd_test_usbip::authenticator_server::client::HybridAdvertisement,
    ) -> Result<()> {
        let relay = keyringd_testkit::PxpAdvertisementRelay::new(self.app().agent_socket()?);
        self.runtime
            .block_on(relay.send(&advertisement.service_data()?))?;
        Ok(())
    }

    type ProfilesWindow<'a>
        = MacosProfilesWindow<'a>
    where
        Self: 'a;
    type ProfileSetupWindow<'a>
        = MacosProfileSetupWindow<'a>
    where
        Self: 'a;
    type AboutWindow<'a>
        = MacosAboutWindow<'a>
    where
        Self: 'a;
    type SshConnectionTestWindow<'a>
        = MacosSshConnectionTestWindow<'a>
    where
        Self: 'a;
    type ConnectionTestResultWindow<'a>
        = MacosConnectionTestResultWindow<'a>
    where
        Self: 'a;
    type QrPromptWindow<'a>
        = MacosQrPromptWindow<'a>
    where
        Self: 'a;
    type SecurityKeyWindow<'a>
        = MacosSecurityKeyWindow<'a>
    where
        Self: 'a;
    type PinPromptWindow<'a>
        = MacosPinPromptWindow<'a>
    where
        Self: 'a;

    fn uses_native_fido(&self) -> Result<bool> {
        Ok(!self.app().state()?.test_passkey_service)
    }
    fn start(&mut self, configuration: crate::application::ApplicationConfiguration) -> Result<()> {
        if let Some(app) = self.app.as_mut() {
            app.start_with_configuration(&configuration, self.authenticator_usb_endpoint)
        } else {
            self.app = Some(TestApp::launch_internal(
                configuration
                    .test_fido_mode
                    .is_none()
                    .then_some(self.authenticator_usb_endpoint),
                configuration.test_fido_mode,
                configuration.accept_remote_hybrid_advertisements,
                configuration.device_key_unavailable,
            )?);
            Ok(())
        }
    }

    fn stop(&mut self) -> Result<()> {
        match self.app.as_mut() {
            Some(app) => app.stop(),
            None => Ok(()),
        }
    }
    fn restart(&mut self) -> Result<()> {
        self.app
            .as_mut()
            .context("application has not been started")?
            .restart()
    }
    fn set_profile_save_failure(&self, fail: bool) -> Result<()> {
        self.app().fail_saves(fail)
    }
    fn set_ssh_client_configuration(
        &self,
        host: &str,
        configuration: &crate::ssh::SshClientConfiguration,
    ) -> Result<()> {
        let agent = if configuration.use_agent {
            self.app().agent_socket()?
        } else {
            "none".into()
        };
        write_ssh_client_configuration(self.app().evidence_directory(), host, configuration, &agent)
    }

    fn profiles_window(&mut self) -> Result<Self::ProfilesWindow<'_>> {
        MacosProfilesWindow::new(&UiDriver::new(self.app()))
    }
    fn profile_setup_window(&mut self) -> Self::ProfileSetupWindow<'_> {
        MacosProfileSetupWindow::new(UiDriver::new(self.app()))
    }
    fn about_window(&mut self) -> Self::AboutWindow<'_> {
        MacosAboutWindow::new(self.app())
    }
    fn ssh_connection_test_window(&mut self) -> Self::SshConnectionTestWindow<'_> {
        MacosSshConnectionTestWindow { app: self.app() }
    }
    fn connection_test_result_window(&mut self) -> Self::ConnectionTestResultWindow<'_> {
        MacosConnectionTestResultWindow { app: self.app() }
    }
    fn qr_prompt_window(&mut self) -> Result<Self::QrPromptWindow<'_>> {
        Ok(MacosQrPromptWindow::new(&UiDriver::new(self.app())))
    }
    fn security_key_window(&mut self) -> Result<Self::SecurityKeyWindow<'_>> {
        Ok(MacosSecurityKeyWindow::new(self.app()))
    }
    fn pin_prompt_window(&mut self) -> Result<Self::PinPromptWindow<'_>> {
        Ok(MacosPinPromptWindow::new(self.app()))
    }

    fn reset_profiles(&mut self) -> Result<()> {
        self.app
            .as_mut()
            .context("application has not been started")?
            .restart_with_empty_profiles()
    }

    fn local_timestamp(&self) -> Result<String> {
        let output = std::process::Command::new("date")
            .arg("+%Y-%m-%d %H:%M:%S")
            .output()?;
        anyhow::ensure!(output.status.success(), "failed to read macOS local time");
        Ok(String::from_utf8(output.stdout)?.trim().to_owned())
    }

    fn agent_endpoint(&self) -> Result<String> {
        self.app().agent_socket()
    }
    fn agent_identities(&self) -> Result<Vec<AgentIdentity>> {
        crate::ssh::macos::list_identities(self.app())?
            .identities
            .lines()
            .map(AgentIdentity::from_openssh)
            .collect()
    }

    fn stored_profiles(&self) -> Result<Vec<serde_json::Value>> {
        self.app().profiles()
    }

    fn connection_requests(&self) -> Result<Option<Vec<SshConnectionRequest>>> {
        Ok(None)
    }

    fn screenshot(&self) -> Result<PathBuf> {
        let ui = UiDriver::new(self.app());
        if ui
            .find_element("profile-setup.heading")?
            .is_some_and(|element| element.hidden != Some(true))
        {
            MacosProfileSetupWizard::current(&ui)?.screenshot()
        } else {
            MacosProfilesWindow::new(&ui)?.screenshot()
        }
    }

    fn drag_supported() -> Result<bool> {
        Ok(true)
    }
}

fn write_ssh_client_configuration(
    home: &Path,
    alias: &str,
    configuration: &crate::ssh::SshClientConfiguration,
    agent: &str,
) -> Result<()> {
    let directory = home.join(".ssh");
    std::fs::create_dir_all(&directory)?;
    std::fs::set_permissions(&directory, std::fs::Permissions::from_mode(0o700))?;
    let text = format!(
        "Host {alias}\n    HostName {}\n    Port {}\n    User {}\n    IdentityAgent {agent}\n    UserKnownHostsFile /dev/null\n    StrictHostKeyChecking no\n",
        configuration.host, configuration.port, configuration.user
    );
    let path = directory.join("config");
    std::fs::write(&path, text)?;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
    Ok(())
}

impl MacosSystemTestApplication {
    pub fn set_test_fido_mode(
        &mut self,
        mode: keyringd_core::test_support::TestFidoMode,
    ) -> Result<()> {
        self.app().set_fido_mode(mode)
    }
}
impl MacosSystemTestApplication {
    pub fn request_exit(&self) -> Result<()> {
        UiDriver::new(self.app()).invoke_status_menu_item("status-menu.quit")
    }
    pub fn wait_exited(&mut self) -> Result<()> {
        self.app
            .as_mut()
            .context("application is not started")?
            .wait_exited()
    }
}

impl MacosSystemTestApplication {
    pub fn ssh_client_configuration_window(
        &self,
    ) -> Result<
        crate::application::features::navigation::macos::MacosSshClientConfigurationWindow<'_>,
    > {
        crate::application::features::navigation::macos::MacosSshClientConfigurationWindow::open(
            &UiDriver::new(self.app()),
        )
    }
    pub fn ssh_agent_selection_enabled(&self) -> Result<bool> {
        let path = self.app().evidence_directory().join("setting.json");
        match std::fs::read(path) {
            Ok(value) => {
                let document = keyringd_core::persistence::SettingDocument::from_json(&value)?;
                match document.values.get("macos.sshAgentSelection") {
                    None => Ok(false),
                    Some(serde_json::Value::String(value)) if value == "keyringd" => Ok(true),
                    Some(serde_json::Value::String(value)) if value == "system" => Ok(false),
                    Some(value) => anyhow::bail!("Invalid saved SSH agent selection: {value}"),
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
            Err(error) => Err(error.into()),
        }
    }

    pub fn original_agent_socket(&self) -> PathBuf {
        self.app().evidence_directory().join("original-agent.sock")
    }
}
