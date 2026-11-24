use std::io::{BufRead, BufReader, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex, mpsc};
use std::time::{Duration, Instant};

use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};

/// Owns only the child launched by this case; never touches a running product instance.
pub struct TestApp {
    child: Arc<Mutex<crate::environment::windows::TestProcess>>,
    stop_watchdog: mpsc::Sender<()>,
    watchdog: Option<std::thread::JoinHandle<()>>,
    _directory: tempfile::TempDir,
    control_address: Option<SocketAddr>,
    registry: crate::environment::windows::RegistrySandbox,
    configuration: crate::application::ApplicationConfiguration,
    stopped_credentials: Option<Value>,
    device_key_ledger: PathBuf,
}

impl TestApp {
    pub fn launch(configuration: &crate::application::ApplicationConfiguration) -> Result<Self> {
        let registry = crate::environment::windows::RegistrySandbox::new();
        let directory = tempfile::tempdir()?;
        let device_key_ledger = std::env::var_os("KEYRINGD_UI_CASE_DIR")
            .map(PathBuf::from)
            .map_or_else(
                || directory.path().join("device-keys"),
                |case| {
                    case.join("device-keys").join(
                        directory
                            .path()
                            .file_name()
                            .expect("temporary directory name"),
                    )
                },
            );
        Self::require_agent_pipe_available()?;
        let child = fixture_command(
            directory.path(),
            registry.key(),
            configuration,
            &device_key_ledger,
        )?
        .spawn()
        .context("failed to launch the UI test application")?;
        let child = Arc::new(Mutex::new(crate::environment::windows::TestProcess::Child(
            child,
        )));
        let watched_child = Arc::clone(&child);
        let (stop_watchdog, receiver) = mpsc::channel();
        let test_service = passkey_backend(configuration) == Some("test-service");
        let watchdog = std::thread::spawn(move || {
            let timeout = if test_service {
                Duration::from_secs(60)
            } else {
                Duration::from_secs(240)
            };
            if receiver.recv_timeout(timeout).is_err()
                && let Ok(mut child) = watched_child.lock()
            {
                let _ = child.kill();
            }
        });
        let mut app = Self {
            child,
            stop_watchdog,
            watchdog: Some(watchdog),
            _directory: directory,
            control_address: None,
            registry,
            configuration: configuration.clone(),
            stopped_credentials: None,
            device_key_ledger,
        };
        app.finish_start()?;
        Ok(app)
    }

    pub fn uses_test_passkey_service(&self) -> bool {
        passkey_backend(&self.configuration) == Some("test-service")
    }

    fn wait_ready(&mut self) -> Result<()> {
        let control_file = self._directory.path().join("control-address");
        let deadline = Instant::now() + Duration::from_secs(15);
        loop {
            if let Ok(error) = std::fs::read_to_string(control_file.with_extension("error")) {
                anyhow::bail!("UI fixture startup failed: {error}");
            }
            // File creation can be observed before its contents have been written.
            if let Ok(address) = std::fs::read_to_string(&control_file)
                && let Ok(address) = address.trim().parse()
            {
                self.control_address = Some(address);
                break;
            }
            if let Some(status) = self.child.lock().unwrap().try_wait()? {
                anyhow::bail!(
                    "UI application exited before the control endpoint was ready: {status}"
                );
            }
            ensure!(
                Instant::now() < deadline,
                "UI control endpoint did not become ready"
            );
            std::thread::sleep(Duration::from_millis(25));
        }
        let pid = self.control(json!({}))?["pid"]
            .as_u64()
            .context("fixture did not expose its PID")?;
        let pid = u32::try_from(pid)?;
        if pid != self.pid() {
            *self.child.lock().unwrap() = crate::environment::windows::TestProcess::activated(pid)?;
        }
        Ok(())
    }

    pub(crate) fn stop(&mut self) -> Result<()> {
        let snapshot = if self.uses_test_passkey_service()
            && self.control_address.is_some()
            && self.is_running()?
        {
            self.control(json!({"export_fido_credentials":true}))
                .map(|credentials| self.stopped_credentials = Some(credentials))
        } else {
            Ok(())
        };
        self.terminate()?;
        snapshot
    }

    fn terminate(&mut self) -> Result<()> {
        let mut child = self.child.lock().unwrap();
        if child.try_wait()?.is_none() {
            child.kill()?;
        }
        child.wait()?;
        self.control_address = None;
        Ok(())
    }

    pub(crate) fn is_running(&self) -> Result<bool> {
        Ok(self.child.lock().unwrap().try_wait()?.is_none())
    }

    fn finish_start(&mut self) -> Result<()> {
        let result = (|| {
            self.wait_ready()?;
            if let Some(mode) = self.configuration.test_fido_mode {
                self.set_fido_mode(mode)?;
            }
            if self.uses_test_passkey_service()
                && let Some(credentials) = &self.stopped_credentials
            {
                self.control(credentials.clone())?;
            }
            Ok(())
        })();
        if let Err(error) = result {
            return match self.terminate() {
                Ok(()) => Err(error),
                Err(cleanup) => Err(error.context(format!(
                    "also failed to stop the partially started fixture: {cleanup:#}"
                ))),
            };
        }
        Ok(())
    }

    pub fn set_fido_mode(&mut self, mode: keyringd_core::test_support::TestFidoMode) -> Result<()> {
        ensure!(
            self.uses_test_passkey_service(),
            "this fixture does not use TestPasskeyService"
        );
        self.control(json!({"fido_mode": serde_json::to_value(mode)?}))?;
        self.configuration.test_fido_mode = Some(mode);
        Ok(())
    }
    pub(crate) fn start_with_configuration(
        &mut self,
        configuration: &crate::application::ApplicationConfiguration,
    ) -> Result<()> {
        ensure!(!self.is_running()?, "application is already started");
        self.configuration = configuration.clone();
        self.spawn(false)
    }

    pub fn restart(&mut self) -> Result<()> {
        self.restart_using(false)
    }

    pub fn restart_with_empty_profiles(&mut self) -> Result<()> {
        self.restart_using(true)
    }

    fn restart_using(&mut self, clear_profiles: bool) -> Result<()> {
        self.stop()?;
        self.spawn(clear_profiles)
    }

    fn spawn(&mut self, clear_profiles: bool) -> Result<()> {
        Self::require_agent_pipe_available()?;
        let control_file = self._directory.path().join("control-address");
        for file in [
            &control_file,
            &control_file.with_extension("error"),
            &control_file.with_extension("pid"),
        ] {
            match std::fs::remove_file(file) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(error.into()),
            }
        }
        let profiles = self._directory.path().join("profiles.json");
        if clear_profiles && profiles.try_exists()? {
            std::fs::remove_file(profiles)?;
        }
        let child = fixture_command(
            self._directory.path(),
            self.registry.key(),
            &self.configuration,
            &self.device_key_ledger,
        )?
        .spawn()?;
        *self.child.lock().unwrap() = crate::environment::windows::TestProcess::Child(child);
        self.control_address = None;
        self.finish_start()
    }

    pub fn configured_agent(&self) -> Result<Option<String>> {
        self.registry.read_agent()
    }

    /// Stop setup without deleting the profile. The next Agent connection must start it.
    pub fn stop_for_socket_activation(&mut self) -> Result<Vec<(String, String)>> {
        let mut child = self.child.lock().unwrap();
        child.kill()?;
        child.wait()?;
        Self::require_agent_pipe_available()?;
        let control_file = self._directory.path().join("control-address");
        std::fs::remove_file(&control_file)?;
        let _ = std::fs::remove_file(control_file.with_extension("error"));
        let _ = std::fs::remove_file(control_file.with_extension("pid"));
        self.control_address = None;
        let mut environment = vec![
            (
                "KEYRINGD_UI_TEST_DEVICE_KEY_LEDGER".to_owned(),
                self.device_key_ledger.to_string_lossy().into_owned(),
            ),
            (
                "KEYRINGD_UI_TEST_SSH_CONFIGURATION_REGISTRY_KEY".to_owned(),
                self.registry.key().to_owned(),
            ),
            (
                "KEYRINGD_UI_TEST_CONTROL_FILE".to_owned(),
                control_file.to_string_lossy().into_owned(),
            ),
            (
                "KEYRINGD_UI_TEST_STATE_FILE".to_owned(),
                self._directory
                    .path()
                    .join("profiles.json")
                    .to_string_lossy()
                    .into_owned(),
            ),
        ];
        if let Some(backend) = passkey_backend(&self.configuration) {
            environment.push((
                "KEYRINGD_UI_TEST_FIDO_BACKEND".to_owned(),
                backend.to_owned(),
            ));
        }
        if let Some(backend) = device_key_backend(&self.configuration) {
            environment.push((
                "KEYRINGD_UI_TEST_DEVICE_KEY_BACKEND".to_owned(),
                backend.to_owned(),
            ));
        }
        for name in [
            "KEYRINGD_UI_TEST_HYBRID_TUNNEL_ADDRESS",
            "KEYRINGD_UI_TEST_HYBRID_TUNNEL_CA",
            "KEYRINGD_UI_TEST_TRAY_GUID",
        ] {
            if let Ok(value) = std::env::var(name) {
                environment.push((name.to_owned(), value));
            }
        }
        Ok(environment)
    }

    /// Adopt only the process exposing this fixture's unique control endpoint.
    pub fn wait_socket_activation(&mut self) -> Result<()> {
        let deadline = Instant::now() + Duration::from_secs(60);
        let control_file = self._directory.path().join("control-address");
        loop {
            if let Ok(pid) = std::fs::read_to_string(control_file.with_extension("pid"))
                && let Ok(pid) = pid.parse::<u32>()
                && pid != self.pid()
            {
                *self.child.lock().unwrap() =
                    crate::environment::windows::TestProcess::activated(pid)?;
            }
            if let Ok(error) = std::fs::read_to_string(control_file.with_extension("error")) {
                anyhow::bail!("socket-activated fixture startup failed: {error}");
            }
            if let Ok(address) = std::fs::read_to_string(&control_file)
                && let Ok(address) = address.trim().parse()
            {
                self.control_address = Some(address);
                let pid = self.control(json!({}))?["pid"]
                    .as_u64()
                    .context("activated fixture did not expose its PID")?;
                *self.child.lock().unwrap() =
                    crate::environment::windows::TestProcess::activated(u32::try_from(pid)?)?;
                return Ok(());
            }
            ensure!(
                Instant::now() < deadline,
                "socket connection did not activate the Windows fixture"
            );
            std::thread::sleep(Duration::from_millis(25));
        }
    }

    pub fn wait_exited(&mut self) -> Result<()> {
        let deadline = Instant::now() + Duration::from_secs(15);
        loop {
            if let Some(status) = self.child.lock().unwrap().try_wait()? {
                self.control_address = None;
                ensure!(status.success(), "fixture exited with {status}");
                return Ok(());
            }
            ensure!(Instant::now() < deadline, "fixture did not exit");
            std::thread::sleep(Duration::from_millis(25));
        }
    }

    pub fn agent_pipe(&self) -> Result<String> {
        self.control(json!({}))?["agent_pipe"]
            .as_str()
            .map(str::to_owned)
            .context("fixture did not expose an Agent pipe")
    }

    pub fn git_for_windows_socket(&self) -> Result<PathBuf> {
        self.wait_control("Git for Windows socket publication", |state| {
            state["git_for_windows_socket"]
                .as_str()
                .is_some_and(|path| std::path::Path::new(path).is_file())
        })?["git_for_windows_socket"]
            .as_str()
            .map(PathBuf::from)
            .context("fixture did not expose a Git for Windows socket")
    }

    pub fn require_agent_pipe_available() -> Result<()> {
        match std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(crate::ssh::windows::CurrentUser::agent_pipe_name()?)
        {
            Err(error) if error.raw_os_error() == Some(2) => Ok(()),
            _ => anyhow::bail!(
                "The keyringd agent pipe is in use or inaccessible. Close the running Windows application before running UI tests."
            ),
        }
    }
    pub fn fido_requests(&self) -> Result<u64> {
        ensure!(
            self.uses_test_passkey_service(),
            "FIDO request counter belongs to the test PasskeyService"
        );
        self.control(json!({}))?["requests"]
            .as_u64()
            .context("fixture did not expose FIDO requests")
    }

    pub fn webauthn_owner(&self) -> Result<usize> {
        self.control(json!({}))?["webauthn_owner"]
            .as_u64()
            .map(|v| v as usize)
            .context("fixture is not using Windows WebAuthn")
    }
    pub fn pid(&self) -> u32 {
        self.child.lock().unwrap().id()
    }

    /// Controls only the service boundary. UI actions always go through UI Automation.
    pub fn control(&self, command: Value) -> Result<Value> {
        let address = self
            .control_address
            .context("this fixture has no service control endpoint")?;
        let mut stream = TcpStream::connect_timeout(&address, Duration::from_secs(3))?;
        stream.set_read_timeout(Some(Duration::from_secs(3)))?;
        stream.set_write_timeout(Some(Duration::from_secs(3)))?;
        writeln!(stream, "{command}")?;
        let mut response = String::new();
        BufReader::new(stream).read_line(&mut response)?;
        let response: Value = serde_json::from_str(&response)?;
        ensure!(
            response.get("error").is_none(),
            "control command failed: {response}"
        );
        Ok(response)
    }

    pub fn wait_control(&self, label: &str, predicate: impl Fn(&Value) -> bool) -> Result<Value> {
        let deadline = Instant::now() + Duration::from_secs(15);
        loop {
            let state = self.control(json!({}))?;
            if predicate(&state) {
                return Ok(state);
            }
            ensure!(
                Instant::now() < deadline,
                "timed out waiting for {label}: {state}"
            );
            std::thread::sleep(Duration::from_millis(25));
        }
    }
}

impl Drop for TestApp {
    fn drop(&mut self) {
        let _ = self.stop_watchdog.send(());
        if let Some(watchdog) = self.watchdog.take() {
            let _ = watchdog.join();
        }
        if !self.uses_test_passkey_service() && self.control_address.is_some() {
            let _ = self.control(json!({"cancel_fido_operations": true}));
            let deadline = Instant::now() + Duration::from_secs(6);
            let mut settled = false;
            while Instant::now() < deadline {
                if self
                    .control(json!({}))
                    .is_ok_and(|state| state["active_fido_operations"].as_u64() == Some(0))
                {
                    settled = true;
                    break;
                }
                std::thread::sleep(Duration::from_millis(25));
            }
            if !settled {
                eprintln!("Windows WebAuthn operation did not settle after cancellation");
            }
            if let Ok(owner) = self.webauthn_owner()
                && let Ok(windows) =
                    crate::application::platform::windows::window_locator::owned_windows(owner)
            {
                for window in windows {
                    unsafe {
                        let _ = windows::Win32::UI::WindowsAndMessaging::PostMessageW(
                            Some(window),
                            windows::Win32::UI::WindowsAndMessaging::WM_CLOSE,
                            windows::Win32::Foundation::WPARAM(0),
                            windows::Win32::Foundation::LPARAM(0),
                        );
                    }
                }
            }
        }
        if let Ok(mut child) = self.child.lock() {
            let _ = child.kill();
            let _ = child.wait();
        }
        if let Some(executable) = std::env::var_os("KEYRINGD_UI_TEST_APP")
            && let Err(error) = Self::cleanup_device_keys(
                std::path::Path::new(&executable),
                &self.device_key_ledger,
            )
        {
            eprintln!("Could not clean up fixture CNG keys: {error:#}");
        }
        // A fallible setup helper may drop the fixture before its caller panics.
        // Keep diagnostics in the captured test output for that path as well.
        if let Ok(log) = std::fs::read_to_string(self._directory.path().join("control-address.log"))
            && !log.trim().is_empty()
        {
            eprintln!("Windows Agent fixture log:\n{log}");
        }
    }
}

fn passkey_backend(
    configuration: &crate::application::ApplicationConfiguration,
) -> Option<&'static str> {
    if configuration.accept_remote_hybrid_advertisements {
        Some("hybrid")
    } else if configuration.test_fido_mode.is_some() {
        Some("test-service")
    } else {
        None
    }
}

fn device_key_backend(
    configuration: &crate::application::ApplicationConfiguration,
) -> Option<&'static str> {
    configuration.software_cng.then_some("cng-software")
}

fn fixture_command(
    directory: &std::path::Path,
    registry_key: &str,
    configuration: &crate::application::ApplicationConfiguration,
    ledger: &std::path::Path,
) -> Result<Command> {
    let executable = std::env::var_os("KEYRINGD_UI_TEST_APP")
        .context("KEYRINGD_UI_TEST_APP must point to the UI test executable")?;
    let mut command = Command::new(executable);
    command
        .arg("--control-file")
        .arg(directory.join("control-address"))
        .arg("--state-file")
        .arg(directory.join("profiles.json"))
        .arg("--device-key-ledger")
        .arg(ledger)
        .arg("--ssh-configuration-registry-key")
        .arg(registry_key)
        .stdin(Stdio::null());
    command.env_remove("KEYRINGD_UI_TEST_FIDO_BACKEND");
    if let Some(backend) = passkey_backend(configuration) {
        command.env("KEYRINGD_UI_TEST_FIDO_BACKEND", backend);
    }
    command.env_remove("KEYRINGD_UI_TEST_DEVICE_KEY_BACKEND");
    if let Some(backend) = device_key_backend(configuration) {
        command.env("KEYRINGD_UI_TEST_DEVICE_KEY_BACKEND", backend);
    }
    command.env_remove("KEYRINGD_UI_TEST_DEVICE_KEY_UNAVAILABLE");
    if configuration.device_key_unavailable {
        command.env("KEYRINGD_UI_TEST_DEVICE_KEY_UNAVAILABLE", "1");
    }
    if configuration.software_cng {
        command.env(
            "RUST_LOG",
            "warn,keyringd_windows::system::device_key=debug",
        );
    }
    Ok(command)
}

impl TestApp {
    /// Runs without the GUI or service endpoint; the ledger survives Profile resets and crashes.
    pub fn cleanup_device_keys(
        executable: &std::path::Path,
        ledger: &std::path::Path,
    ) -> Result<()> {
        use std::os::windows::process::CommandExt;
        if !ledger.try_exists()? || std::fs::read_dir(ledger)?.next().is_none() {
            return Ok(());
        }
        let log = tempfile::tempfile()?;
        let mut child = Command::new(executable)
            .creation_flags(0x08000000)
            .arg("--cleanup-device-keys")
            .arg(ledger)
            .stdin(Stdio::null())
            .stdout(Stdio::from(log.try_clone()?))
            .stderr(Stdio::from(log.try_clone()?))
            .spawn()?;
        let deadline = Instant::now() + Duration::from_secs(30);
        let status = loop {
            match child.try_wait() {
                Ok(Some(status)) => break status,
                Ok(None) if Instant::now() < deadline => {
                    std::thread::sleep(Duration::from_millis(25))
                }
                other => {
                    let _ = child.kill();
                    let _ = child.wait();
                    match other {
                        Err(error) => return Err(error.into()),
                        _ => anyhow::bail!(
                            "case CNG cleanup timed out; ledger retained at {}",
                            ledger.display()
                        ),
                    }
                }
            }
        };
        use std::io::{Read, Seek, SeekFrom};
        let mut log = log;
        log.seek(SeekFrom::Start(0))?;
        let mut message = String::new();
        log.read_to_string(&mut message)?;
        ensure!(
            status.success(),
            "case CNG cleanup failed ({status}): {message}; ledger retained at {}",
            ledger.display()
        );
        Ok(())
    }
}
