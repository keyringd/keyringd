use crate::application::features::pin_prompt_window::linux::LinuxPinPromptWindow;
use crate::application::features::ssh_connection_test::SshConnectionRequest;
use crate::application::features::{
    about::linux::LinuxAboutWindow,
    profile_setup::linux::LinuxProfileSetupWindow,
    profiles::linux::LinuxProfilesWindow,
    qr_prompt_window::linux::LinuxQrPromptWindow,
    security_key_window::linux::LinuxSecurityKeyWindow,
    ssh_connection_test::linux::{LinuxConnectionTestResultWindow, LinuxSshConnectionTestWindow},
};

use crate::application::platform::linux::{ELEMENT_TIMEOUT, UiDriver};
use crate::environment::linux::Environment;
use anyhow::{Result, ensure};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use tokio::runtime::Runtime;

use crate::application::SystemTestApplication;

pub struct LinuxSystemTestApplication {
    environment: Environment,
    running: bool,
    unavailable_tpm: Option<PathBuf>,
    runtime: std::sync::Arc<Runtime>,
    desktop: UiDriver,
    session: UiDriver,
}
impl LinuxSystemTestApplication {
    pub fn device_key_security_window(&self) -> Result<crate::application::features::device_key_security_window::linux::LinuxDeviceKeySecurityWindow<'_>>{
        Ok(crate::application::features::device_key_security_window::linux::LinuxDeviceKeySecurityWindow {
            runtime: &self.runtime,
            session: self.session.clone(),
        })
    }
    pub fn agent_journal(&self) -> Result<String> {
        self.runtime.block_on(self.environment.agent_journal())
    }
    pub(crate) fn new(environment: Environment, runtime: std::sync::Arc<Runtime>) -> Result<Self> {
        let session = runtime.block_on(UiDriver::connect(&environment))?;
        Ok(Self {
            environment,
            running: false,
            unavailable_tpm: None,
            runtime,
            desktop: session.clone(),
            session,
        })
    }
    async fn launch(&self) -> Result<UiDriver> {
        self.environment.launch_installed_application().await?;
        let profiles = self
            .desktop
            .wait_for_visible_name("SSH Profiles", std::time::Duration::from_secs(20))
            .await?;
        let session = self.desktop.scoped_to_application(&profiles).await?;
        let profiles = session
            .wait_for_group("SSH Profiles", ELEMENT_TIMEOUT)
            .await?;
        profiles
            .wait_for_visible_action("Create Profile", ELEMENT_TIMEOUT)
            .await?;
        Ok(session)
    }
}
impl SystemTestApplication for LinuxSystemTestApplication {
    fn send_hybrid_advertisement(
        &self,
        advertisement: &keyringd_test_usbip::authenticator_server::client::HybridAdvertisement,
    ) -> Result<()> {
        let relay = keyringd_testkit::PxpAdvertisementRelay::new(self.environment.agent_socket());
        self.runtime
            .block_on(relay.send(&advertisement.service_data()?))?;
        Ok(())
    }

    type ProfilesWindow<'a>
        = LinuxProfilesWindow<'a>
    where
        Self: 'a;
    type ProfileSetupWindow<'a>
        = LinuxProfileSetupWindow<'a>
    where
        Self: 'a;
    type AboutWindow<'a>
        = LinuxAboutWindow<'a>
    where
        Self: 'a;
    type SshConnectionTestWindow<'a>
        = LinuxSshConnectionTestWindow<'a>
    where
        Self: 'a;
    type ConnectionTestResultWindow<'a>
        = LinuxConnectionTestResultWindow<'a>
    where
        Self: 'a;
    type QrPromptWindow<'a>
        = LinuxQrPromptWindow<'a>
    where
        Self: 'a;
    type SecurityKeyWindow<'a>
        = LinuxSecurityKeyWindow<'a>
    where
        Self: 'a;
    type PinPromptWindow<'a>
        = LinuxPinPromptWindow<'a>
    where
        Self: 'a;

    fn uses_native_fido(&self) -> Result<bool> {
        // Linux application startup only supports the real FIDO backend.
        Ok(true)
    }
    fn start(&mut self, configuration: crate::application::ApplicationConfiguration) -> Result<()> {
        ensure!(!self.running, "application is already started");
        ensure!(
            configuration.test_fido_mode.is_none(),
            "Linux uses the native FIDO backend"
        );
        if configuration.device_key_unavailable {
            let path = self
                .environment
                .home()?
                .join(".config/systemd/user/keyringd-desktop.service.d/unavailable-tpm.conf");
            std::fs::create_dir_all(path.parent().unwrap())?;
            self.unavailable_tpm = Some(path.clone());
            std::fs::write(
                &path,
                "[Service]\nEnvironment=TPM2TOOLS_TCTI=device:/dev/keyringd-test-missing-tpm\nEnvironment=TCTI=device:/dev/keyringd-test-missing-tpm\nEnvironment=TEST_TCTI=device:/dev/keyringd-test-missing-tpm\n",
            )?;
            let output = self
                .runtime
                .block_on(self.environment.systemctl_user(&["daemon-reload"]))?;
            ensure!(
                output.status.success(),
                "TPM test configuration reload failed: {}",
                output.stderr
            );
        }
        // A failed readiness check still owns the process started for this case.
        self.running = true;
        let started = self.runtime.block_on(self.launch());
        match started {
            Ok(session) => self.session = session,
            Err(error) => {
                if let Err(cleanup) = self.stop() {
                    return Err(error.context(format!(
                        "also failed to stop the partially started application: {cleanup:#}"
                    )));
                }
                return Err(error);
            }
        }
        Ok(())
    }
    fn stop(&mut self) -> Result<()> {
        self.runtime
            .block_on(self.environment.stop_installed_application())?;
        if let Some(path) = &self.unavailable_tpm {
            match std::fs::remove_file(path) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(error.into()),
            }
            let output = self
                .runtime
                .block_on(self.environment.systemctl_user(&["daemon-reload"]))?;
            ensure!(
                output.status.success(),
                "TPM test configuration restore failed: {}",
                output.stderr
            );
            self.unavailable_tpm = None;
        }
        self.running = false;
        // Socket activation can start a new GUI process without start(). Keep
        // the shared desktop connection, but discard the stopped process scope.
        self.session = self.desktop.clone();
        Ok(())
    }
    fn restart(&mut self) -> Result<()> {
        self.stop()?;
        self.start(Default::default())
    }
    fn reset_profiles(&mut self) -> Result<()> {
        self.stop()?;
        self.runtime
            .block_on(self.environment.reset_installed_application())?;
        self.running = false;
        self.start(Default::default())
    }

    fn set_profile_save_failure(&self, fail: bool) -> Result<()> {
        let store = self.environment.root().join("data/keyringd/profiles.json");
        if fail {
            std::fs::create_dir_all(store)?;
        } else {
            std::fs::remove_dir(store)?;
        }
        Ok(())
    }
    fn set_ssh_client_configuration(
        &self,
        host: &str,
        configuration: &crate::ssh::SshClientConfiguration,
    ) -> Result<()> {
        let agent = if configuration.use_agent {
            self.environment
                .agent_socket()
                .to_string_lossy()
                .into_owned()
        } else {
            "none".into()
        };
        write_ssh_client_configuration(&self.environment.home()?, host, configuration, &agent)
    }

    fn profiles_window(&mut self) -> Result<Self::ProfilesWindow<'_>> {
        LinuxProfilesWindow::new(&self.runtime, self.session.clone())
    }
    fn profile_setup_window(&mut self) -> Self::ProfileSetupWindow<'_> {
        LinuxProfileSetupWindow {
            runtime: &self.runtime,
            session: self.session.clone(),
        }
    }
    fn about_window(&mut self) -> Self::AboutWindow<'_> {
        LinuxAboutWindow {
            runtime: &self.runtime,
            session: self.session.clone(),
        }
    }
    fn ssh_connection_test_window(&mut self) -> Self::SshConnectionTestWindow<'_> {
        LinuxSshConnectionTestWindow {
            runtime: &self.runtime,
            session: self.session.clone(),
        }
    }
    fn connection_test_result_window(&mut self) -> Self::ConnectionTestResultWindow<'_> {
        LinuxConnectionTestResultWindow {
            runtime: &self.runtime,
            session: self.session.clone(),
        }
    }
    fn qr_prompt_window(&mut self) -> Result<Self::QrPromptWindow<'_>> {
        Ok(LinuxQrPromptWindow {
            runtime: &self.runtime,
            session: self.session.clone(),
        })
    }
    fn security_key_window(&mut self) -> Result<Self::SecurityKeyWindow<'_>> {
        Ok(LinuxSecurityKeyWindow {
            runtime: &self.runtime,
            session: self.session.clone(),
        })
    }
    fn pin_prompt_window(&mut self) -> Result<Self::PinPromptWindow<'_>> {
        Ok(LinuxPinPromptWindow {
            runtime: &self.runtime,
            session: self.session.clone(),
        })
    }

    fn local_timestamp(&self) -> Result<String> {
        let output = self
            .runtime
            .block_on(self.environment.run("date", &["+%Y-%m-%d %H:%M:%S"]))?;
        ensure!(output.status.success(), "failed to read Linux local time");
        Ok(output.stdout.trim().to_owned())
    }

    fn agent_endpoint(&self) -> Result<String> {
        Ok(self
            .environment
            .agent_socket()
            .to_string_lossy()
            .into_owned())
    }
    fn agent_identities(&self) -> Result<Vec<keyringd_testkit::agent::AgentIdentity>> {
        self.runtime
            .block_on(self.environment.list_agent_identities())
    }

    fn stored_profiles(&self) -> Result<Vec<serde_json::Value>> {
        self.environment.stored_profiles()
    }

    fn connection_requests(&self) -> Result<Option<Vec<SshConnectionRequest>>> {
        Ok(None)
    }

    fn screenshot(&self) -> Result<PathBuf> {
        self.runtime.block_on(self.session.capture())
    }

    fn drag_supported() -> Result<bool> {
        Ok(true)
    }
}

impl Drop for LinuxSystemTestApplication {
    fn drop(&mut self) {
        if let Err(error) = self.stop() {
            eprintln!("failed to stop the case-owned application: {error:#}");
        }
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

impl LinuxSystemTestApplication {
    pub fn ssh_client_configuration_window(
        &self,
    ) -> Result<crate::application::features::navigation::linux::LinuxSshClientConfigurationWindow>
    {
        crate::application::features::navigation::linux::LinuxSshClientConfigurationWindow::open(
            std::sync::Arc::clone(&self.runtime),
            self.session.clone(),
        )
    }
}
