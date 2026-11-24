use super::SecurityAutomationAction;
use super::security_automation_request::SecurityAutomationRequest;
use crate::application::platform::windows::UiDriver;
use anyhow::{Result, ensure};
use serde_json::Value;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::os::windows::io::{FromRawHandle, OwnedHandle};
use std::path::Path;
use std::time::Duration;
use windows::Win32::Foundation::{HWND, WAIT_TIMEOUT};
use windows::Win32::System::Threading::{OpenProcess, PROCESS_SYNCHRONIZE, WaitForSingleObject};
use windows::Win32::UI::WindowsAndMessaging::{IsWindowVisible, PostMessageW, WM_CLOSE};

/// The runner's process handle bounds the elevation's lifetime even if it crashes.
pub fn serve(ready_file: &Path, token: &str, parent_pid: u32, report: &Path) -> Result<()> {
    let parent = unsafe { OpenProcess(PROCESS_SYNCHRONIZE, false, parent_pid)? };
    let _parent_owner = unsafe { OwnedHandle::from_raw_handle(parent.0) };
    let listener = TcpListener::bind(("127.0.0.1", 0))?;
    listener.set_nonblocking(true)?;
    // Only this dedicated process writes to its own environment.
    unsafe { std::env::set_var("KEYRINGD_TEST_REPORT_DIR", report) };
    let driver = UiDriver::new()?;
    let pending = ready_file.with_extension("pending");
    std::fs::write(&pending, listener.local_addr()?.to_string())?;
    std::fs::rename(pending, ready_file)?;
    while unsafe { WaitForSingleObject(parent, 0) } == WAIT_TIMEOUT {
        let (mut stream, _) = match listener.accept() {
            Ok(connection) => connection,
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(50));
                continue;
            }
            Err(error) => return Err(error.into()),
        };
        // Windows accepted sockets inherit the listener's nonblocking mode.
        // Receive each request with a bounded blocking read; TCP may deliver
        // its JSON payload and terminating newline in separate packets.
        stream.set_nonblocking(false)?;
        stream.set_read_timeout(Some(Duration::from_secs(5)))?;
        stream.set_write_timeout(Some(Duration::from_secs(5)))?;
        let mut line = String::new();
        let read = BufReader::new((&mut stream).take(64 * 1024)).read_line(&mut line);
        let mut shutdown = false;
        let result = (|| -> Result<Value> {
            read?;
            let request: SecurityAutomationRequest = serde_json::from_str(&line)?;
            ensure!(request.token == token, "invalid broker token");
            if matches!(request.action, SecurityAutomationAction::Shutdown) {
                shutdown = true;
                return Ok(Value::Null);
            }
            match &request.action {
                SecurityAutomationAction::IsElevated => {
                    return Ok(serde_json::json!(
                        crate::environment::windows::process_elevation::is_elevated()?
                    ));
                }
                SecurityAutomationAction::WaitWebAuthnOpen => {
                    driver.webauthn_window()?;
                    return Ok(Value::Null);
                }
                SecurityAutomationAction::WaitWebAuthnClosed => {
                    driver.wait_webauthn_closed()?;
                    return Ok(Value::Null);
                }
                SecurityAutomationAction::CaptureWebAuthn => {
                    return Ok(serde_json::to_value(
                        driver.capture(&driver.webauthn_window()?)?,
                    )?);
                }
                SecurityAutomationAction::WebAuthnEditors => {
                    return Ok(driver.webauthn_pin_editor_count()?.into());
                }
                SecurityAutomationAction::EnterWebAuthnPin { value } => {
                    driver.enter_webauthn_pin(value)?;
                    return Ok(Value::Null);
                }
                _ => {}
            }
            let profile = &request.profile_name;
            ensure!(!profile.is_empty(), "profile name is required");
            let description = format!("TPM-protected SSH key for the \"{profile}\" profile");
            match request.action {
                SecurityAutomationAction::WaitOpen => {
                    return Ok(driver.wait_security_window(&description)?.into());
                }
                SecurityAutomationAction::WaitClosed { window_handle } => {
                    driver.wait("CNG dialog closed", || {
                        Ok(
                            (!unsafe { IsWindowVisible(HWND(window_handle as *mut _)) }.as_bool())
                                .then_some(()),
                        )
                    })?;
                }
                SecurityAutomationAction::Capture => {
                    let root =
                        driver.element_from_handle(driver.wait_security_window(&description)?)?;
                    return Ok(serde_json::to_value(driver.capture(&root)?)?);
                }
                SecurityAutomationAction::WaitEditors { count } => {
                    driver.wait_security_password_fields(&description, count)?;
                }
                SecurityAutomationAction::CreatePassword { value } => {
                    let root = driver.wait_security_password_fields(&description, 2)?;
                    driver.type_security_edit(&root, 0, &value)?;
                    let root = driver.wait_security_password_fields(&description, 2)?;
                    driver.type_security_edit(&root, 1, &value)?;
                    let root = driver.wait_security_password_fields(&description, 2)?;
                    driver.submit_security_dialog(&root)?;
                }
                SecurityAutomationAction::EnterPassword { value } => {
                    let root = driver.wait_security_password_fields(&description, 1)?;
                    driver.type_security_edit(&root, 0, &value)?;
                    let root = driver.wait_security_password_fields(&description, 1)?;
                    driver.submit_security_dialog(&root)?;
                }
                SecurityAutomationAction::WaitPasswordRejected => {
                    return Ok(driver.wait_security_password_rejected(&description)?.into());
                }
                SecurityAutomationAction::Dismiss => {
                    let matches = driver.security_windows_with_description(&description)?;
                    ensure!(matches.len() <= 1, "multiple CNG dialogs for {profile:?}");
                    if let Some(handle) = matches.first() {
                        unsafe {
                            PostMessageW(
                                Some(HWND(*handle as *mut _)),
                                WM_CLOSE,
                                Default::default(),
                                Default::default(),
                            )?
                        };
                    }
                }
                SecurityAutomationAction::IsElevated
                | SecurityAutomationAction::Shutdown
                | SecurityAutomationAction::WaitWebAuthnOpen
                | SecurityAutomationAction::WaitWebAuthnClosed
                | SecurityAutomationAction::CaptureWebAuthn
                | SecurityAutomationAction::WebAuthnEditors
                | SecurityAutomationAction::EnterWebAuthnPin { .. } => unreachable!(),
            }
            Ok(Value::Null)
        })()
        .map_err(|error| {
            crate::environment::windows::process_security_diagnostics::log(
                "Security Automation broker failure",
                std::process::id(),
            );
            let mut diagnostics = Vec::new();
            for handle in driver.windows_security_handles().unwrap_or_default() {
                let mut pid = 0;
                unsafe {
                    windows::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId(
                        HWND(handle as *mut _),
                        Some(&mut pid),
                    );
                }
                crate::environment::windows::process_security_diagnostics::log(
                    "Windows Security failure",
                    pid,
                );
                diagnostics.push(format!(
                    "{}; inventory: {:?}",
                    driver.window_diagnostics(handle),
                    driver
                        .element_from_handle(handle)
                        .and_then(|root| driver.control_inventory(&root))
                ));
            }
            let message = format!("{error:#}; Windows Security diagnostics: {diagnostics:?}");
            if let Ok(mut log) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(report.join("security-automation.log"))
            {
                let _ = writeln!(log, "{message}");
            }
            message
        });
        // A disconnected client must not tear down the reusable broker.
        if serde_json::to_writer(&mut stream, &result).is_ok() {
            let _ = stream.write_all(b"\n");
        }
        if shutdown {
            break;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::authenticator::windows::security_automation::SecurityAutomationClient;
    use std::net::TcpStream;
    use std::time::Instant;

    keyringd_testkit::scenario! {
        group "serve" {
            case "Broker waits for a request delivered in parts" {
                verify "read the complete USB broker request across multiple writes" {
                    let directory = tempfile::tempdir().unwrap();
                    let ready = directory.path().join("ready");
                    let report = directory.path().to_owned();
                    let server_ready = ready.clone();
                    let server =
                        std::thread::spawn(move || serve(&server_ready, "test-token", std::process::id(), &report));
                    let deadline = Instant::now() + Duration::from_secs(10);
                    let address = loop {
                        if let Ok(address) = std::fs::read_to_string(&ready) {
                            break address.parse::<std::net::SocketAddr>().unwrap();
                        }
                        if server.is_finished() {
                            let result = server.join().expect("broker worker panicked");
                            panic!("broker exited before becoming ready: {result:?}");
                        }
                        assert!(Instant::now() < deadline, "broker did not become ready");
                        std::thread::sleep(Duration::from_millis(10));
                    };
                    let request = serde_json::to_vec(&SecurityAutomationRequest {
                        token: "test-token".into(),
                        profile_name: String::new(),
                        action: SecurityAutomationAction::Shutdown,
                    })
                    .unwrap();
                    let mut stream = TcpStream::connect(address).unwrap();
                    stream
                        .set_read_timeout(Some(Duration::from_secs(5)))
                        .unwrap();
                    stream.write_all(&request[..1]).unwrap();
                    std::thread::sleep(Duration::from_millis(200));
                    let delivery = stream
                        .write_all(&request[1..])
                        .and_then(|()| stream.write_all(b"\n"));
                    let mut response = String::new();
                    let received = BufReader::new(stream).read_line(&mut response);
                    // If the request was rejected, shut down the broker before reporting
                    // the regression so the test does not leave a worker running.
                    if !server.is_finished() {
                        let _ = SecurityAutomationClient::new(address, "test-token".into())
                            .unwrap()
                            .call("", SecurityAutomationAction::Shutdown);
                    }
                    server.join().unwrap().unwrap();
                    delivery.unwrap();
                    received.unwrap();
                    let result: std::result::Result<Value, String> = serde_json::from_str(&response).unwrap();
                    assert_eq!(result.unwrap(), Value::Null);
                }
            }
        }
    }
}
