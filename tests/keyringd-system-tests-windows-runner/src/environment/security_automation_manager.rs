use anyhow::{Context, Result, ensure};
use keyringd_system_tests::authenticator::windows::security_automation::SecurityAutomationAction;
use keyringd_system_tests::authenticator::windows::security_automation::SecurityAutomationClient;
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
use std::path::Path;
use std::time::{Duration, Instant};
use windows::Win32::Foundation::{HANDLE, WAIT_TIMEOUT};
use windows::Win32::System::Threading::{TerminateProcess, WaitForSingleObject};
use windows::Win32::UI::Shell::{SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW, ShellExecuteExW};
use windows::Win32::UI::WindowsAndMessaging::SW_HIDE;
use windows::core::PCWSTR;

/// Caches one elevation for the entire run, including all scenario processes.
/// No elevated service or approval state is retained after the runner exits.
pub(crate) struct SecurityAutomationManager {
    process: OwnedHandle,
    client: SecurityAutomationClient,
    address: String,
    token: String,
    _directory: tempfile::TempDir,
}
impl SecurityAutomationManager {
    pub fn start(executable: &Path, report: &Path) -> Result<Self> {
        let directory = tempfile::tempdir()?;
        let ready = directory.path().join("ready");
        let token = uuid::Uuid::new_v4().to_string();
        let executable: Vec<u16> = executable
            .as_os_str()
            .encode_wide()
            .chain(Some(0))
            .collect();
        let verb: Vec<u16> = "runas".encode_utf16().chain(Some(0)).collect();
        // Windows paths cannot contain quotes; neither argument ends in a slash.
        let parameters: Vec<u16> = format!(
            "\"{}\" {} {} \"{}\"",
            ready.display(),
            token,
            std::process::id(),
            report.display()
        )
        .encode_utf16()
        .chain(Some(0))
        .collect();
        let mut info = SHELLEXECUTEINFOW {
            cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
            fMask: SEE_MASK_NOCLOSEPROCESS,
            lpVerb: PCWSTR(verb.as_ptr()),
            lpFile: PCWSTR(executable.as_ptr()),
            lpParameters: PCWSTR(parameters.as_ptr()),
            nShow: SW_HIDE.0,
            ..Default::default()
        };
        println!("Windows Security Automation: requesting UAC approval once for this run");
        unsafe { ShellExecuteExW(&mut info) }
            .context("Windows Security Automation elevation failed or was cancelled")?;
        ensure!(
            !info.hProcess.is_invalid(),
            "broker launch returned no process handle"
        );
        let process = unsafe { OwnedHandle::from_raw_handle(info.hProcess.0) };
        let deadline = Instant::now() + Duration::from_secs(30);
        let address = loop {
            if let Ok(address) = std::fs::read_to_string(&ready) {
                break address;
            }
            let alive =
                unsafe { WaitForSingleObject(HANDLE(process.as_raw_handle()), 0) } == WAIT_TIMEOUT;
            if !alive || Instant::now() >= deadline {
                unsafe {
                    let _ = TerminateProcess(HANDLE(process.as_raw_handle()), 1);
                }
                anyhow::bail!("Windows Security Automation broker did not become ready");
            }
            std::thread::sleep(Duration::from_millis(50));
        };
        let client = SecurityAutomationClient::new(address.parse()?, token.clone())?;
        Ok(Self {
            process,
            client,
            address,
            token,
            _directory: directory,
        })
    }
    pub fn configure(&self, command: &mut std::process::Command) {
        command
            .env("KEYRINGD_SECURITY_AUTOMATION_ADDRESS", &self.address)
            .env("KEYRINGD_SECURITY_AUTOMATION_TOKEN", &self.token);
    }
}
impl Drop for SecurityAutomationManager {
    fn drop(&mut self) {
        let _ = self.client.call("", SecurityAutomationAction::Shutdown);
        let process = HANDLE(self.process.as_raw_handle());
        if unsafe { WaitForSingleObject(process, 2000) } == WAIT_TIMEOUT {
            unsafe {
                let _ = TerminateProcess(process, 1);
            }
            unsafe {
                WaitForSingleObject(process, 2000);
            }
        }
    }
}
use std::os::windows::ffi::OsStrExt;
