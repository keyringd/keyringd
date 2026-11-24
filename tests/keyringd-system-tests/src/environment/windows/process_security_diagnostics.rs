use anyhow::{Result, ensure};
use std::io::Write;
use std::os::windows::io::{FromRawHandle, OwnedHandle};
use windows::Win32::Foundation::HANDLE;
use windows::Win32::Security::*;
use windows::Win32::System::RemoteDesktop::ProcessIdToSessionId;
use windows::Win32::System::Threading::{
    OpenProcess, OpenProcessToken, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
    QueryFullProcessImageNameW,
};

/// Diagnostic failures must not change the scenario's outcome.
pub fn log(label: &str, pid: u32) {
    let message = match describe(pid) {
        Ok(details) => format!("Process security: {label}: {details}"),
        Err(error) => format!("Process security: {label}: PID {pid}: {error:#}"),
    };
    write_log(&message);
}

fn write_log(message: &str) {
    eprintln!("{message}");
    if let Some(report) = std::env::var_os("KEYRINGD_TEST_REPORT_DIR")
        && let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(std::path::PathBuf::from(report).join("process-security.log"))
    {
        let _ = writeln!(file, "{message}");
    }
}

pub(crate) fn describe(pid: u32) -> Result<String> {
    unsafe {
        let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid)?;
        let _process = OwnedHandle::from_raw_handle(process.0);
        let mut token = Default::default();
        OpenProcessToken(process, TOKEN_QUERY, &mut token)?;
        let _token = OwnedHandle::from_raw_handle(token.0);
        let mut executable = vec![0u16; 32768];
        let mut executable_length = executable.len() as u32;
        let executable = QueryFullProcessImageNameW(
            process,
            PROCESS_NAME_WIN32,
            windows::core::PWSTR(executable.as_mut_ptr()),
            &mut executable_length,
        )
        .map(|()| String::from_utf16_lossy(&executable[..executable_length as usize]));
        let mut session = 0;
        ProcessIdToSessionId(pid, &mut session)?;
        Ok(format!(
            "PID {pid}, {}, session={session}, executable={executable:?}",
            describe_token(token)?
        ))
    }
}

fn describe_token(token: HANDLE) -> Result<String> {
    unsafe {
        let mut ui_access = 0u32;
        let mut length = 0;
        GetTokenInformation(
            token,
            TokenUIAccess,
            Some((&mut ui_access as *mut u32).cast()),
            size_of::<u32>() as u32,
            &mut length,
        )?;
        let mut elevation = TOKEN_ELEVATION::default();
        let mut elevation_type = TOKEN_ELEVATION_TYPE::default();
        let mut length = 0;
        GetTokenInformation(
            token,
            TokenElevation,
            Some((&mut elevation as *mut TOKEN_ELEVATION).cast()),
            std::mem::size_of_val(&elevation) as u32,
            &mut length,
        )?;
        GetTokenInformation(
            token,
            TokenElevationType,
            Some((&mut elevation_type as *mut TOKEN_ELEVATION_TYPE).cast()),
            std::mem::size_of_val(&elevation_type) as u32,
            &mut length,
        )?;
        let mut linked = TOKEN_LINKED_TOKEN::default();
        let linked_token = match GetTokenInformation(
            token,
            TokenLinkedToken,
            Some((&mut linked as *mut TOKEN_LINKED_TOKEN).cast()),
            size_of::<TOKEN_LINKED_TOKEN>() as u32,
            &mut length,
        ) {
            Ok(()) => {
                let _linked = OwnedHandle::from_raw_handle(linked.LinkedToken.0);
                "present".to_owned()
            }
            Err(error) => format!("unavailable({error})"),
        };
        length = 0;
        let _ = GetTokenInformation(token, TokenIntegrityLevel, None, 0, &mut length);
        ensure!(
            length as usize >= std::mem::size_of::<TOKEN_MANDATORY_LABEL>(),
            "invalid integrity label size"
        );
        let mut buffer = vec![0usize; (length as usize).div_ceil(std::mem::size_of::<usize>())];
        GetTokenInformation(
            token,
            TokenIntegrityLevel,
            Some(buffer.as_mut_ptr().cast()),
            length,
            &mut length,
        )?;
        let label = &*buffer.as_ptr().cast::<TOKEN_MANDATORY_LABEL>();
        let count = *GetSidSubAuthorityCount(label.Label.Sid);
        ensure!(count > 0, "empty integrity SID");
        let integrity = *GetSidSubAuthority(label.Label.Sid, u32::from(count - 1));
        let name = match integrity {
            0x0000 => "Untrusted",
            0x1000 => "Low",
            0x2000 => "Medium",
            0x2100 => "MediumPlus",
            0x3000 => "High",
            0x4000 => "System",
            _ => "Other",
        };
        Ok(format!(
            "elevated={}, elevation_type={}, integrity={name}(0x{integrity:x}), ui_access={ui_access}, linked_token={linked_token}",
            elevation.TokenIsElevated != 0,
            elevation_type.0
        ))
    }
}
