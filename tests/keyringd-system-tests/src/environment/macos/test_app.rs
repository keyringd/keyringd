use keyringd_core::test_support::TestFidoMode;
use std::cell::Cell;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, ensure};
use keyringd_testkit::macos_ui_protocol::{ControlCommand, ControlResponse, FixtureState};
use serde_json::Value;

/// Owns only the fixture this case launched. A product instance the developer
/// is running is never located, signalled or killed.
pub struct TestApp {
    child: Mutex<Child>,
    directory: tempfile::TempDir,
    control_address: Option<SocketAddr>,
    authenticator_usb: Option<SocketAddr>,
    test_fido_mode: Option<Cell<TestFidoMode>>,
    stopped_credentials: Option<Vec<Value>>,
    accept_remote_hybrid_advertisements: bool,
    device_key_unavailable: bool,
    _original_agent: std::os::unix::net::UnixListener,
}

impl TestApp {
    pub(crate) fn launch_internal(
        usb_address: Option<SocketAddr>,
        test_fido_mode: Option<TestFidoMode>,
        accept_remote_hybrid_advertisements: bool,
        device_key_unavailable: bool,
    ) -> Result<Self> {
        let directory = tempfile::Builder::new()
            .prefix("p4s-ui-")
            .tempdir_in("/tmp")?;
        let original_agent =
            std::os::unix::net::UnixListener::bind(directory.path().join("original-agent.sock"))?;
        Self::register_directory(directory.path())?;
        let child = Mutex::new(
            match spawn(
                directory.path(),
                usb_address,
                test_fido_mode,
                accept_remote_hybrid_advertisements,
                device_key_unavailable,
            ) {
                Ok(child) => child,
                Err(error) => {
                    Self::save_directory_diagnostics(directory.path());
                    return Err(error);
                }
            },
        );
        let mut app = Self {
            child,
            directory,
            control_address: None,
            authenticator_usb: usb_address,

            test_fido_mode: test_fido_mode.map(Cell::new),
            stopped_credentials: None,
            _original_agent: original_agent,
            accept_remote_hybrid_advertisements,
            device_key_unavailable,
        };
        if let Err(error) = app.wait_ready() {
            app.save_diagnostics();
            return Err(error);
        }
        Ok(app)
    }

    /// Stops this fixture and starts another one on the same state file, so a
    /// case can check what the product persisted. The user's own application
    /// support directory is still never involved.
    pub(crate) fn stop(&mut self) -> Result<()> {
        let snapshot = if self.test_fido_mode.is_some()
            && self.control_address.is_some()
            && self.is_running()?
        {
            self.state()
                .map(|state| self.stopped_credentials = Some(state.fido_credentials))
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
    pub(crate) fn start_with_configuration(
        &mut self,
        configuration: &crate::application::ApplicationConfiguration,
        authenticator_usb_endpoint: SocketAddr,
    ) -> Result<()> {
        ensure!(!self.is_running()?, "application is already started");
        self.authenticator_usb = configuration
            .test_fido_mode
            .is_none()
            .then_some(authenticator_usb_endpoint);
        self.test_fido_mode = configuration.test_fido_mode.map(Cell::new);
        self.device_key_unavailable = configuration.device_key_unavailable;
        self.accept_remote_hybrid_advertisements =
            configuration.accept_remote_hybrid_advertisements;
        self.restart()
    }

    pub fn restart(&mut self) -> Result<()> {
        self.restart_using(false, spawn)
    }

    /// Simulates another machine, retaining credentials in AuthenticatorServer.
    pub fn restart_with_empty_profiles(&mut self) -> Result<()> {
        self.restart_using(true, spawn)
    }

    fn restart_using(
        &mut self,
        clear_profiles: bool,
        spawn_fixture: impl FnOnce(
            &std::path::Path,
            Option<SocketAddr>,
            Option<TestFidoMode>,
            bool,
            bool,
        ) -> Result<Child>,
    ) -> Result<()> {
        // Native credentials remain in AuthenticatorServer; only mock credentials are transferred.
        self.stop()?;
        let result = (|| {
            remove_if_present(&self.directory.path().join("control-address"))?;
            if clear_profiles {
                remove_if_present(&self.directory.path().join("profiles.json"))?;
            }
            let child = spawn_fixture(
                self.directory.path(),
                self.authenticator_usb,
                self.test_fido_mode.as_ref().map(Cell::get),
                self.accept_remote_hybrid_advertisements,
                self.device_key_unavailable,
            )?;
            *self.child.lock().unwrap() = child;
            self.wait_ready()?;
            if self.test_fido_mode.is_some()
                && let Some(credentials) = &self.stopped_credentials
            {
                self.change(ControlCommand {
                    fido_credentials: Some(credentials.clone()),
                    ..Default::default()
                })?;
            }
            Ok(())
        })();
        if let Err(error) = result {
            self.save_diagnostics();
            return match self.terminate() {
                Ok(()) => Err(error),
                Err(cleanup) => Err(error.context(format!(
                    "also failed to stop the partially started fixture: {cleanup:#}"
                ))),
            };
        }
        Ok(())
    }

    fn is_running(&self) -> Result<bool> {
        Ok(self.child.lock().unwrap().try_wait()?.is_none())
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

    fn wait_ready(&mut self) -> Result<()> {
        self.wait_ready_with_timeout(Duration::from_secs(20))
    }

    fn wait_ready_with_timeout(&mut self, timeout: Duration) -> Result<()> {
        use keyringd_testkit::macos_ui_protocol::{Scope, Selector, Target, UiCommand, UiResponse};
        let control_file = self.directory.path().join("control-address");
        let deadline = Instant::now() + timeout;
        let mut last = "control address not published".to_owned();
        loop {
            if let Some(status) = self.child.lock().unwrap().try_wait()? {
                anyhow::bail!("the fixture exited before its UI was ready: {status}");
            }
            if let Ok(address) = std::fs::read_to_string(&control_file)
                && let Ok(address) = address.trim().parse()
            {
                let command = ControlCommand {
                    ui: Some(UiCommand::FindElement {
                        target: Target {
                            scope: Scope::Application,
                            selector: Selector::Id("profiles.configure-ssh-clients".into()),
                        },
                    }),
                    ..Default::default()
                };
                match crate::environment::macos::diagnostics::exchange(
                    address,
                    &command,
                    Duration::from_millis(250),
                ) {
                    Ok(ControlResponse::Ui {
                        result: UiResponse::Elements { elements },
                    }) => {
                        ensure!(elements.len() <= 1, "ambiguous Profiles window anchor");
                        if elements
                            .first()
                            .is_some_and(|element| element.hidden != Some(true))
                        {
                            self.control_address = Some(address);
                            ensure!(
                                self.state()?.test_passkey_service == self.test_fido_mode.is_some(),
                                "fixture started with the wrong FIDO backend"
                            );
                            return Ok(());
                        }
                        last = "Profiles window not visible".to_owned();
                    }
                    Ok(other) => anyhow::bail!("unexpected startup response: {other:?}"),
                    Err(error) => last = format!("{error:#}"),
                }
            }
            ensure!(
                Instant::now() < deadline,
                "fixture UI did not become ready: {last}"
            );
            std::thread::sleep(Duration::from_millis(25));
        }
    }

    /// Controls only the service boundary. UI actions always go through the
    /// accessibility driver, never through this.
    pub(crate) fn control(&self, command: ControlCommand) -> Result<ControlResponse> {
        command.validate().map_err(anyhow::Error::msg)?;
        let address = self
            .control_address
            .context("this fixture has no control endpoint")?;
        crate::environment::macos::diagnostics::exchange(address, &command, Duration::from_secs(10))
    }

    pub fn state(&self) -> Result<FixtureState> {
        match self.control(ControlCommand::default())? {
            ControlResponse::State { state } => Ok(state),
            other => anyhow::bail!("expected fixture state, got {other:?}"),
        }
    }
    pub fn fido_requests(&self) -> Result<u64> {
        ensure!(
            self.test_fido_mode.is_some(),
            "native request counts belong to the explicit Security Key; this API requires TestPasskeyService"
        );
        Ok(self.state()?.requests)
    }
    pub fn profiles(&self) -> Result<Vec<Value>> {
        Ok(self.state()?.profiles)
    }
    pub fn agent_socket(&self) -> Result<String> {
        Ok(self.state()?.agent_socket)
    }
    pub fn set_fido_mode(&self, mode: TestFidoMode) -> Result<()> {
        let current = self
            .test_fido_mode
            .as_ref()
            .context("this fixture uses AuthenticatorServer, not TestPasskeyService")?;
        self.change(ControlCommand {
            fido_mode: Some(serde_json::to_value(mode)?),
            ..Default::default()
        })?;
        current.set(mode);
        Ok(())
    }
    pub fn fail_saves(&self, fail: bool) -> Result<()> {
        self.change(ControlCommand {
            fail_saves: Some(fail),
            ..Default::default()
        })
    }
    fn change(&self, command: ControlCommand) -> Result<()> {
        match self.control(command)? {
            ControlResponse::State { .. } => Ok(()),
            other => anyhow::bail!("expected state acknowledgement, got {other:?}"),
        }
    }

    /// Polls the fixture state until `predicate` returns true, with a
    /// 10-second timeout.
    pub fn wait_control(
        &self,
        description: &str,
        predicate: impl Fn(&FixtureState) -> bool,
    ) -> Result<FixtureState> {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            let state = self.state()?;
            if predicate(&state) {
                return Ok(state);
            }
            ensure!(
                Instant::now() < deadline,
                "timed out waiting for {description}; last state: {state:?}"
            );
            std::thread::sleep(Duration::from_millis(25));
        }
    }

    fn register_directory(directory: &std::path::Path) -> Result<()> {
        if let Some(case) = std::env::var_os("KEYRINGD_UI_CASE_DIR") {
            let case = PathBuf::from(case);
            std::fs::create_dir_all(case.join("fixtures"))?;
            let name = directory
                .file_name()
                .context("fixture directory has no name")?;
            std::fs::write(
                case.join("fixtures").join(name).with_extension("json"),
                serde_json::to_vec(&crate::environment::macos::diagnostics::FixtureRecord {
                    directory: directory.into(),
                })?,
            )?;
        }
        Ok(())
    }

    fn save_diagnostics(&self) {
        Self::save_directory_diagnostics(self.directory.path());
    }

    fn save_directory_diagnostics(directory: &std::path::Path) {
        let Some(report) = std::env::var_os("KEYRINGD_TEST_REPORT_DIR") else {
            return;
        };
        let base = std::env::var_os("KEYRINGD_UI_CASE_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(report));
        let name = directory.file_name().unwrap_or_default();
        let destination = base.join("diagnostics").join(name);
        if let Err(error) = crate::environment::macos::diagnostics::collect(directory, &destination)
        {
            eprintln!("failed to save fixture diagnostics: {error:#}");
        }
    }

    /// The case-private state directory. Evidence copied to the report survives
    /// cleanup; transient state is removed when this fixture is dropped.
    pub fn evidence_directory(&self) -> &std::path::Path {
        self.directory.path()
    }
}

fn remove_if_present(path: &std::path::Path) -> Result<()> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

fn spawn(
    directory: &std::path::Path,
    usb_address: Option<SocketAddr>,
    test_fido_mode: Option<TestFidoMode>,
    accept_remote_hybrid_advertisements: bool,
    device_key_unavailable: bool,
) -> Result<Child> {
    fixture_command(
        directory,
        usb_address,
        test_fido_mode,
        accept_remote_hybrid_advertisements,
        device_key_unavailable,
    )?
    .spawn()
    .context("failed to launch the UI test fixture")
}

fn fixture_command(
    directory: &std::path::Path,
    usb_address: Option<SocketAddr>,
    test_fido_mode: Option<TestFidoMode>,
    accept_remote_hybrid_advertisements: bool,
    device_key_unavailable: bool,
) -> Result<Command> {
    let executable = std::env::var_os("KEYRINGD_UI_TEST_APP")
        .context("KEYRINGD_UI_TEST_APP must point to the UI test fixture executable")?;
    let mut command = Command::new(PathBuf::from(executable));
    command.env_remove("KEYRINGD_UI_TEST_DEVICE_KEY_UNAVAILABLE");
    if device_key_unavailable {
        command.env("KEYRINGD_UI_TEST_DEVICE_KEY_UNAVAILABLE", "1");
    }
    command
        .arg("--control-file")
        .arg(directory.join("control-address"))
        .arg("--state-file")
        .arg(directory.join("profiles.json"))
        .stdin(Stdio::null());
    if let Some(address) = usb_address {
        command.arg("--authenticator-usb").arg(address.to_string());
    }
    if let Some(mode) = test_fido_mode {
        command.arg("--test-fido-mode").arg(
            serde_json::to_value(mode)?
                .as_str()
                .context("FIDO mode must serialize as a string")?,
        );
    }
    if accept_remote_hybrid_advertisements {
        command.arg("--accept-remote-hybrid-advertisements");
    }
    let log = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(directory.join("app.log"))?;
    command
        .stdout(Stdio::from(log.try_clone()?))
        .stderr(Stdio::from(log));
    Ok(command)
}

impl Drop for TestApp {
    fn drop(&mut self) {
        if std::thread::panicking() {
            self.save_diagnostics();
        }
        if let Ok(mut child) = self.child.lock() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::platform::macos::UiDriver;
    use std::io::{BufRead, BufReader, Write};
    use std::net::TcpListener;

    fn fixture(address: SocketAddr) -> TestApp {
        let directory = tempfile::tempdir_in("/tmp").unwrap();
        let original_agent =
            std::os::unix::net::UnixListener::bind(directory.path().join("original-agent.sock"))
                .unwrap();
        TestApp {
            child: Mutex::new(Command::new("sleep").arg("30").spawn().unwrap()),
            directory,
            control_address: Some(address),
            authenticator_usb: None,
            test_fido_mode: Some(Cell::new(TestFidoMode::Success)),
            stopped_credentials: None,
            accept_remote_hybrid_advertisements: false,
            device_key_unavailable: false,
            _original_agent: original_agent,
        }
    }

    fn with_response(response: Option<&'static str>, verify: impl FnOnce(&UiDriver<'_>)) {
        with_response_checked(response, |_| {}, verify);
    }

    fn with_response_checked(
        response: Option<&'static str>,
        check: impl FnOnce(&ControlCommand) + Send + 'static,
        verify: impl FnOnce(&UiDriver<'_>),
    ) {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            let mut request = String::new();
            BufReader::new(socket.try_clone().unwrap())
                .read_line(&mut request)
                .unwrap();
            let request: ControlCommand = serde_json::from_str(&request).unwrap();
            check(&request);
            if let Some(response) = response {
                writeln!(socket, "{response}").unwrap();
            }
        });
        let app = fixture(address);
        verify(&UiDriver::new(&app));
        server.join().unwrap();
    }

    keyringd_testkit::scenario! {
        group "UiDriver::wait_absent" {
            case "Absence requires a successful rpc" {
                verify "report a protocol error when the presence query fails" {
                    with_response(
                        Some(r#"{"kind":"ui","result":{"kind":"elements","elements":[]}}"#),
                        |driver| {
                            driver.wait_absent("sheet").unwrap();
                        },
                    );
                    with_response(
                        Some(r#"{"kind":"error","message":"AppKit main thread timed out"}"#),
                        |driver| {
                            assert!(driver.wait_absent("sheet").is_err());
                        },
                    );
                    with_response(None, |driver| {
                        assert!(driver.wait_absent("sheet").is_err());
                    });
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "UiDriver::wait_for" {
            case "Waiting for appearance propagates protocol errors" {
                verify "fail the wait immediately when the accessibility query returns an error" {
                    with_response(
                        Some(r#"{"kind":"ui","result":{"kind":"elements","elements":[{"hidden":"no"}]}}"#),
                        |driver| {
                            assert!(driver.wait_for("sheet").is_err());
                        },
                    );
                }
            }
        }
    }
    keyringd_testkit::scenario! {
        group "UiDriver::find_element" {
            case "Scoped queries preserve the requested window and sheet" {
                verify "preserve the requested window and sheet in the accessibility request" {
                    use keyringd_testkit::macos_ui_protocol::{Scope, Selector, UiCommand};
                    with_response_checked(
                        Some(r#"{"kind":"ui","result":{"kind":"elements","elements":[]}}"#),
                        |command| match command.ui.as_ref().unwrap() {
                            UiCommand::FindElement { target } => {
                                assert_eq!(
                                    target.scope,
                                    Scope::Window {
                                        anchor: "profiles.configure-ssh-clients".into()
                                    }
                                );
                                assert_eq!(
                                    target.selector,
                                    Selector::Id("profiles.row.123.name".into())
                                );
                            }
                            _ => panic!("wrong command"),
                        },
                        |driver| {
                            assert!(
                                driver
                                    .in_window("profiles.configure-ssh-clients")
                                    .find_element("profiles.row.123.name")
                                    .unwrap()
                                    .is_none()
                            );
                        },
                    );
                    with_response_checked(
                        Some(r#"{"kind":"ui","result":{"kind":"pressed","accepted":false}}"#),
                        |command| match command.ui.as_ref().unwrap() {
                            UiCommand::Press { target } => {
                                assert_eq!(
                                    target.scope,
                                    Scope::Sheet {
                                        anchor: "connection-test.host".into()
                                    }
                                );
                                assert_eq!(
                                    target.selector,
                                    Selector::Id("connection-test.result.close".into())
                                );
                            }
                            _ => panic!("wrong command"),
                        },
                        |driver| {
                            driver
                                .in_sheet("connection-test.host")
                                .press("connection-test.result.close")
                                .unwrap();
                        },
                    );
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "UiDriver::find_element" {
            case "Duplicate matches and wrong response types are errors" {
                verify "reject multiple element matches and incorrect RPC response variants" {
                    with_response(
                        Some(
                            r#"{"kind":"ui","result":{"kind":"elements","elements":[{"class":"NSButton"},{"class":"NSButton"}]}}"#,
                        ),
                        |driver| {
                            assert!(
                                driver
                                    .find_element("ambiguous")
                                    .unwrap_err()
                                    .to_string()
                                    .contains("ambiguous")
                            );
                        },
                    );
                    with_response(
                        Some(r#"{"kind":"ui","result":{"kind":"done"}}"#),
                        |driver| {
                            assert!(driver.wait_absent("sheet").is_err());
                        },
                    );
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "UiDriver::wait_with_timeout" {
            case "Wait deadline preserves last observation and ax tree" {
                verify "include the latest observation and AX tree when the wait expires" {
                    with_response(
                        Some(r#"{"kind":"ui","result":{"kind":"tree","tree":"Profiles only; About absent"}}"#),
                        |driver| {
                            let error = driver
                                .wait_with_timeout("About version", Duration::ZERO, &mut || {
                                    Ok((None::<()>, "version element missing".into()))
                                })
                                .unwrap_err();
                            let diagnostic = error.to_string();
                            assert!(diagnostic.contains("About version"));
                            assert!(diagnostic.contains("version element missing"));
                            assert!(diagnostic.contains("Profiles only; About absent"));
                        },
                    );
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "UiDriver::wait_with_timeout" {
            case "Wait deadline preserves diagnostic rpc failure" {
                verify "include the error from the diagnostic RPC when the wait expires" {
                    with_response(None, |driver| {
                        let error = driver
                            .wait_with_timeout("a copied value", Duration::ZERO, &mut || {
                                Ok((None::<()>, "pasteboard is empty".into()))
                            })
                            .unwrap_err();
                        let diagnostic = error.to_string();
                        assert!(diagnostic.contains("pasteboard is empty"));
                        assert!(diagnostic.contains("AX tree unavailable"));
                    });
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "spawn" {
            case "Identity selection waits until the radio button is selected" {
                verify "wait for the accessibility state to reflect selection of the identity" {
                    use crate::application::features::ssh_connection_test::{
                        SshConnectionTestWindowDriver, macos::MacosSshConnectionTestWindow,
                    };
                    use keyringd_testkit::macos_ui_protocol::{Scope, Selector, UiCommand, UiResponse};
                    let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
                    let app = fixture(listener.local_addr().unwrap());
                    let server = std::thread::spawn(move || {
                        let observations = [
                            ("connection-test.host", None),
                            ("connection-test.identity.passkey", None),
                            ("connection-test.identity.passkey", Some("0")),
                            ("connection-test.identity.passkey", Some("1")),
                            ("connection-test.host", None),
                            ("connection-test.identity.device-certificate", Some("0")),
                            ("connection-test.identity.device-key", Some("0")),
                            ("connection-test.identity.passkey", Some("1")),
                        ];
                        for (index, (identifier, value)) in observations.into_iter().enumerate() {
                            let (mut stream, _) = listener.accept().unwrap();
                            let command = request(&stream).ui.unwrap();
                            let target = match &command {
                                UiCommand::FindElement { target } | UiCommand::Press { target } => target,
                                _ => panic!("unexpected command: {command:?}"),
                            };
                            assert_eq!(
                                target.scope,
                                Scope::Window {
                                    anchor: "connection-test.host".into()
                                }
                            );
                            assert_eq!(target.selector, Selector::Id(identifier.into()));
                            let result = if index == 1 {
                                assert!(matches!(command, UiCommand::Press { .. }));
                                UiResponse::Pressed { accepted: false }
                            } else {
                                assert!(matches!(command, UiCommand::FindElement { .. }));
                                UiResponse::Elements { elements: vec![serde_json::from_value(serde_json::json!({"class": "NSButton", "hidden": false, "value": value})).unwrap()] }
                            };
                            answer(&mut stream, &ControlResponse::Ui { result });
                        }
                    });
                    let page = MacosSshConnectionTestWindow {
                        app: &app,
                    };
                    page.select_identity(
                        crate::application::features::ssh_connection_test::SshConnectionIdentity::Passkey,
                    )
                    .unwrap();
                    server.join().unwrap();
                }
            }
        }
    }

    fn startup_state(mock: bool, credentials: Vec<Value>) -> ControlResponse {
        ControlResponse::State {
            state: FixtureState {
                agent_socket: "/tmp/test-agent".into(),
                test_passkey_service: mock,
                requests: 0,
                pending: 0,
                cancelled: 0,
                profiles: vec![],
                fido_credentials: credentials,
            },
        }
    }

    fn answer(stream: &mut std::net::TcpStream, response: &ControlResponse) {
        writeln!(stream, "{}", serde_json::to_string(response).unwrap()).unwrap();
    }

    fn request(stream: &std::net::TcpStream) -> ControlCommand {
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        let mut line = String::new();
        BufReader::new(stream.try_clone().unwrap())
            .read_line(&mut line)
            .unwrap();
        serde_json::from_str(&line).unwrap()
    }

    fn visible_heading() -> ControlResponse {
        use keyringd_testkit::macos_ui_protocol::UiResponse;
        ControlResponse::Ui {
            result: UiResponse::Elements {
                elements: vec![
                    serde_json::from_value(serde_json::json!({"class": "NSView", "hidden": false}))
                        .unwrap(),
                ],
            },
        }
    }

    keyringd_testkit::scenario! {
        group "TestApp::wait_ready_with_timeout" {
            case "A control file without a responsive ui is not ready" {
                verify "reject fixture startup when the control file exists but the UI cannot answer" {
                    let address = "127.0.0.1:0".parse().unwrap();
                    let mut app = fixture(address);
                    app.control_address = None;
                    std::fs::write(
                        app.directory.path().join("control-address"),
                        address.to_string(),
                    )
                    .unwrap();
                    let error = app
                        .wait_ready_with_timeout(Duration::from_millis(50))
                        .unwrap_err();
                    assert!(
                        error
                            .to_string()
                            .contains("fixture UI did not become ready")
                    );
                    assert!(app.control_address.is_none());
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "TestApp::wait_ready_with_timeout" {
            case "Startup waits for the visible profiles window" {
                verify "complete startup only after the Profiles window is visible" {
                    use keyringd_testkit::macos_ui_protocol::{UiCommand, UiResponse};
                    let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
                    let mut app = fixture(listener.local_addr().unwrap());
                    app.control_address = None;
                    std::fs::write(
                        app.directory.path().join("control-address"),
                        listener.local_addr().unwrap().to_string(),
                    )
                    .unwrap();
                    let server = std::thread::spawn(move || {
                        for ready in [false, true] {
                            let (mut stream, _) = listener.accept().unwrap();
                            assert!(matches!(
                                request(&stream).ui,
                                Some(UiCommand::FindElement { .. })
                            ));
                            let response = if ready {
                                visible_heading()
                            } else {
                                ControlResponse::Ui {
                                    result: UiResponse::Elements { elements: vec![] },
                                }
                            };
                            answer(&mut stream, &response);
                        }
                        let (mut stream, _) = listener.accept().unwrap();
                        assert!(request(&stream).ui.is_none());
                        answer(&mut stream, &startup_state(true, vec![]));
                    });
                    app.wait_ready_with_timeout(Duration::from_secs(2)).unwrap();
                    assert!(app.control_address.is_some());
                    server.join().unwrap();
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "TestApp::restart_using" {
            case "Native restart preserves the server and never exports or imports mock credentials" {
                verify "preserve the native authenticator server across restart without mock credential RPCs" {
                    let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
                    let address = listener.local_addr().unwrap();
                    // The old endpoint is deliberately unreachable. Native restart must
                    // work without asking the old process for mock credential snapshots.
                    let mut app = fixture("127.0.0.1:0".parse().unwrap());
                    app.test_fido_mode = None;
                    app.accept_remote_hybrid_advertisements = true;
                    let usb = "127.0.0.1:3240".parse().unwrap();
                    app.authenticator_usb = Some(usb);
                    std::fs::write(app.directory.path().join("profiles.json"), "persisted").unwrap();
                    let server = std::thread::spawn(move || {
                        let (mut stream, _) = listener.accept().unwrap();
                        assert!(request(&stream).ui.is_some());
                        answer(&mut stream, &visible_heading());
                        let (mut stream, _) = listener.accept().unwrap();
                        assert!(request(&stream).fido_credentials.is_none());
                        answer(&mut stream, &startup_state(false, vec![]));
                    });
                    app.restart_using(true, |directory, actual_usb, mode, relay, unavailable| {
                        assert!(relay);
                        assert!(!unavailable);
                        assert_eq!(actual_usb, Some(usb));
                        assert!(mode.is_none());
                        assert!(!directory.join("profiles.json").exists());
                        std::fs::write(directory.join("control-address"), address.to_string())?;
                        Ok(Command::new("sleep").arg("30").spawn()?)
                    })
                    .unwrap();
                    server.join().unwrap();
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "TestApp::restart_using" {
            case "Mock restart restores credentials and the selected mode" {
                verify "import the saved credentials and restore the mock backend after restart" {
                    let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
                    let address = listener.local_addr().unwrap();
                    let mut app = fixture(address);
                    app.test_fido_mode.as_ref().unwrap().set(TestFidoMode::Hold);
                    std::fs::write(app.directory.path().join("profiles.json"), "persisted").unwrap();
                    let credentials = vec![serde_json::json!({"credential_id": "fixture"})];
                    let server = std::thread::spawn(move || {
                        let (mut stream, _) = listener.accept().unwrap();
                        assert!(request(&stream).ui.is_none());
                        answer(&mut stream, &startup_state(true, credentials.clone()));
                        let (mut stream, _) = listener.accept().unwrap();
                        assert!(request(&stream).ui.is_some());
                        answer(&mut stream, &visible_heading());
                        let (mut stream, _) = listener.accept().unwrap();
                        assert!(request(&stream).fido_credentials.is_none());
                        answer(&mut stream, &startup_state(true, vec![]));
                        let (mut stream, _) = listener.accept().unwrap();
                        assert_eq!(request(&stream).fido_credentials, Some(credentials));
                        answer(&mut stream, &startup_state(true, vec![]));
                    });
                    app.restart_using(false, |directory, usb, mode, relay, unavailable| {
                        assert!(!relay);
                        assert!(!unavailable);
                        assert!(usb.is_none());
                        assert_eq!(mode, Some(TestFidoMode::Hold));
                        assert_eq!(
                            std::fs::read_to_string(directory.join("profiles.json"))?,
                            "persisted"
                        );
                        std::fs::write(directory.join("control-address"), address.to_string())?;
                        Ok(Command::new("sleep").arg("30").spawn()?)
                    })
                    .unwrap();
                    server.join().unwrap();
                }
            }
        }
    }
}
