use crate::application::features::navigation::windows::UiApplication;
use crate::application::platform::windows::UiDriver;
use std::path::PathBuf;

use anyhow::{Context, Result};
use keyringd_testkit::agent::AgentIdentity;

use crate::application::features::ssh_connection_test::SshConnectionRequest;
use crate::application::features::{
    about::windows::WindowsAboutWindow,
    profile_setup::windows::WindowsProfileSetupWindow,
    profiles::windows::WindowsProfilesWindow,
    qr_prompt_window::windows::WindowsQrPromptWindow,
    security_key_window::windows::WindowsSecurityKeyWindow,
    ssh_connection_test::windows::{
        WindowsConnectionTestResultWindow, WindowsSshConnectionTestWindow,
    },
};

use crate::environment::windows::TestApp;

use crate::application::SystemTestApplication;

pub struct WindowsSystemTestApplication {
    app: Option<TestApp>,
    driver: UiDriver,
}
impl WindowsSystemTestApplication {
    pub(crate) fn new() -> Result<Self> {
        Ok(Self {
            app: None,
            driver: UiDriver::new()?,
        })
    }
    pub fn test_fido_request_count(&self) -> Result<u64> {
        self.app().fido_requests()
    }

    pub fn application_menu(
        &self,
    ) -> Result<crate::application::features::navigation::windows::ApplicationMenuPage<'_>> {
        UiApplication::new(&self.driver, self.app().pid()).open_main_menu()
    }
    pub fn ssh_client_configuration_window(
        &self,
    ) -> Result<
        crate::application::features::navigation::windows::WindowsSshClientConfigurationWindow<'_>,
    > {
        crate::application::features::navigation::windows::WindowsSshClientConfigurationWindow::current(
            &self.driver,
            self.app().pid(),
        )
    }
    pub fn trust_entries_window(
        &self,
        name: &str,
    ) -> Result<crate::application::features::profiles::windows::TrustEntriesWindow<'_>> {
        UiApplication::new(&self.driver, self.app().pid()).public_keys(name)
    }
    pub fn configured_agent(&self) -> Result<Option<String>> {
        self.app().configured_agent()
    }
    pub fn wait_exited(&mut self) -> Result<()> {
        self.app
            .as_mut()
            .context("application is not started")?
            .wait_exited()
    }
    fn app(&self) -> &TestApp {
        self.app
            .as_ref()
            .expect("application must be started before its features are used")
    }
}
impl SystemTestApplication for WindowsSystemTestApplication {
    fn send_hybrid_advertisement(
        &self,
        advertisement: &keyringd_test_usbip::authenticator_server::client::HybridAdvertisement,
    ) -> Result<()> {
        keyringd_testkit::PxpAdvertisementRelay::new(self.app().agent_pipe()?)
            .send(&advertisement.service_data()?)?;
        Ok(())
    }

    type ProfilesWindow<'a>
        = WindowsProfilesWindow<'a>
    where
        Self: 'a;
    type ProfileSetupWindow<'a>
        = WindowsProfileSetupWindow<'a>
    where
        Self: 'a;
    type AboutWindow<'a>
        = WindowsAboutWindow<'a>
    where
        Self: 'a;
    type SshConnectionTestWindow<'a>
        = WindowsSshConnectionTestWindow<'a>
    where
        Self: 'a;
    type ConnectionTestResultWindow<'a>
        = WindowsConnectionTestResultWindow<'a>
    where
        Self: 'a;
    type QrPromptWindow<'a>
        = WindowsQrPromptWindow<'a>
    where
        Self: 'a;
    type SecurityKeyWindow<'a>
        = WindowsSecurityKeyWindow
    where
        Self: 'a;

    fn uses_native_fido(&self) -> Result<bool> {
        Ok(
            !self.app().control(serde_json::json!({}))?["test_passkey_service"]
                .as_bool()
                .context("missing test FIDO service observation")?,
        )
    }
    fn start(&mut self, configuration: crate::application::ApplicationConfiguration) -> Result<()> {
        if let Some(app) = self.app.as_mut() {
            app.start_with_configuration(&configuration)
        } else {
            self.app = Some(TestApp::launch(&configuration)?);
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
    fn reset_profiles(&mut self) -> Result<()> {
        self.app
            .as_mut()
            .context("application has not been started")?
            .restart_with_empty_profiles()
    }
    fn set_profile_save_failure(&self, fail: bool) -> Result<()> {
        self.app()
            .control(serde_json::json!({"fail_saves": fail}))?;
        Ok(())
    }
    fn set_ssh_client_configuration(
        &self,
        host: &str,
        configuration: &crate::ssh::SshClientConfiguration,
    ) -> Result<()> {
        let text = std::fs::read_to_string(&configuration.config_file)?;
        let agent = if configuration.use_agent {
            self.app().agent_pipe()?
        } else {
            "none".into()
        }
        .replace('\\', "/");
        let text = text
            .lines()
            .map(|line| {
                if line.trim_start().starts_with("IdentityAgent ") {
                    format!("    IdentityAgent {agent}")
                } else if line.trim_start().starts_with("Host ") {
                    format!("Host {host}")
                } else if line.trim_start().starts_with("HostName ") {
                    format!("    HostName {}", configuration.host)
                } else if line.trim_start().starts_with("Port ") {
                    format!("    Port {}", configuration.port)
                } else if line.trim_start().starts_with("User ") {
                    format!("    User {}", configuration.user)
                } else {
                    line.to_owned()
                }
            })
            .collect::<Vec<_>>()
            .join("\n");
        std::fs::write(&configuration.config_file, text)?;
        self.app()
            .control(serde_json::json!({"ssh_config": configuration.config_file}))?;
        Ok(())
    }

    fn profiles_window(&mut self) -> Result<Self::ProfilesWindow<'_>> {
        UiApplication::new(&self.driver, self.app().pid()).profiles_window()
    }
    fn profile_setup_window(&mut self) -> Self::ProfileSetupWindow<'_> {
        WindowsProfileSetupWindow::new(&self.driver, self.app().pid())
    }
    fn about_window(&mut self) -> Self::AboutWindow<'_> {
        WindowsAboutWindow::new(&self.driver, self.app().pid())
    }
    fn ssh_connection_test_window(&mut self) -> Self::SshConnectionTestWindow<'_> {
        WindowsSshConnectionTestWindow::new(self.app(), &self.driver)
    }
    fn connection_test_result_window(&mut self) -> Self::ConnectionTestResultWindow<'_> {
        WindowsConnectionTestResultWindow::new(self.app(), &self.driver)
    }
    fn qr_prompt_window(&mut self) -> Result<Self::QrPromptWindow<'_>> {
        Ok(WindowsQrPromptWindow::new(&self.driver, self.app().pid()))
    }
    fn security_key_window(&mut self) -> Result<Self::SecurityKeyWindow<'_>> {
        WindowsSecurityKeyWindow::new()
    }

    fn local_timestamp(&self) -> Result<String> {
        use std::os::windows::process::CommandExt;
        let output = std::process::Command::new("powershell")
            .args([
                "-NoProfile",
                "-Command",
                "(Get-Date).ToString('yyyy-MM-dd HH:mm:ss')",
            ])
            .creation_flags(0x08000000)
            .output()?;
        anyhow::ensure!(output.status.success(), "failed to read Windows local time");
        Ok(String::from_utf8(output.stdout)?.trim().to_owned())
    }

    fn agent_endpoint(&self) -> Result<String> {
        self.app().agent_pipe()
    }
    fn agent_identities(&self) -> Result<Vec<AgentIdentity>> {
        crate::ssh::windows::agent_identities(self.app())?
            .iter()
            .map(|line| AgentIdentity::from_openssh(line))
            .collect()
    }

    fn stored_profiles(&self) -> Result<Vec<serde_json::Value>> {
        self.app().control(serde_json::json!({}))?["profiles"]
            .as_array()
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("fixture has no profiles array"))
    }

    fn connection_requests(&self) -> Result<Option<Vec<SshConnectionRequest>>> {
        let state = self.app().control(serde_json::json!({}))?;
        let requests = state["ssh_requests"]
            .as_array()
            .context("missing SSH service requests")?;
        Ok(Some(
            requests
                .iter()
                .map(|r| {
                    Ok(SshConnectionRequest {
                        host: r["host"].as_str().context("missing request host")?.into(),
                        public_identity: r["public_identity"]
                            .as_str()
                            .context("missing request identity")?
                            .into(),
                    })
                })
                .collect::<Result<_>>()?,
        ))
    }

    fn screenshot(&self) -> Result<PathBuf> {
        UiApplication::new(&self.driver, self.app().pid()).screenshot()
    }

    fn drag_supported() -> Result<bool> {
        Ok(!crate::environment::windows::process_elevation::is_elevated()?)
    }

    fn wait_for_recovery_idle(&self) -> Result<()> {
        self.app()
            .wait_control("completed recovery requests", |state| state["pending"] == 0)?;
        Ok(())
    }

    fn wait_for_fido_operations_to_finish(&self) -> Result<()> {
        self.app()
            .wait_control("cancelled FIDO operation", |state| {
                state["active_fido_operations"] == 0
            })?;
        Ok(())
    }
}

impl WindowsSystemTestApplication {
    pub fn set_test_fido_mode(
        &mut self,
        mode: keyringd_core::test_support::TestFidoMode,
    ) -> Result<()> {
        self.app
            .as_mut()
            .context("application has not been started")?
            .set_fido_mode(mode)
    }
}
impl WindowsSystemTestApplication {
    pub fn request_exit(&self) -> Result<()> {
        UiApplication::new(&self.driver, self.app().pid())
            .open_main_menu()?
            .exit()
    }
}
impl WindowsSystemTestApplication {
    pub fn git_for_windows_agent_socket(&self) -> Result<PathBuf> {
        self.app().git_for_windows_socket()
    }
}
