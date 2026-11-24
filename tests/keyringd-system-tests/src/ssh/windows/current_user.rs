use anyhow::{Result, ensure};
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
use windows::Win32::Foundation::{HANDLE, HLOCAL, LocalFree};
use windows::Win32::Security::Authorization::ConvertSidToStringSidW;
use windows::Win32::Security::{
    GetTokenInformation, LookupAccountSidW, PSID, SID_NAME_USE, TOKEN_QUERY, TOKEN_USER, TokenUser,
};
use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};
use windows::Win32::System::WindowsProgramming::GetComputerNameW;
use windows::core::{PCWSTR, PWSTR};

pub(crate) struct CurrentUser {
    buffer: Vec<usize>,
}

impl CurrentUser {
    pub(super) fn new() -> Result<Self> {
        let mut token = HANDLE::default();
        unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) }?;
        let token = unsafe { OwnedHandle::from_raw_handle(token.0) };
        let token = HANDLE(token.as_raw_handle());
        let mut length = 0;
        let _ = unsafe { GetTokenInformation(token, TokenUser, None, 0, &mut length) };
        ensure!(
            length >= size_of::<TOKEN_USER>() as u32,
            "could not size current user token"
        );
        let mut buffer = vec![0usize; (length as usize).div_ceil(size_of::<usize>())];
        unsafe {
            GetTokenInformation(
                token,
                TokenUser,
                Some(buffer.as_mut_ptr().cast()),
                length,
                &mut length,
            )
        }?;
        Ok(Self { buffer })
    }

    pub(crate) fn agent_pipe_name() -> Result<String> {
        let sid = Self::new()?.sid_string()?;
        Ok(format!(r"\\.\pipe\keyringd-agent-{sid}"))
    }

    fn sid(&self) -> PSID {
        unsafe { &*self.buffer.as_ptr().cast::<TOKEN_USER>() }
            .User
            .Sid
    }

    pub(super) fn sid_string(&self) -> Result<String> {
        let mut sid = PWSTR::null();
        unsafe { ConvertSidToStringSidW(self.sid(), &mut sid) }?;
        let text = unsafe { sid.to_string() };
        unsafe {
            let _ = LocalFree(Some(HLOCAL(sid.0.cast())));
        }
        Ok(text?)
    }

    pub(super) fn ssh_account_name(&self) -> Result<String> {
        let mut name_length = 0;
        let mut domain_length = 0;
        let mut kind = SID_NAME_USE::default();
        let _ = unsafe {
            LookupAccountSidW(
                PCWSTR::null(),
                self.sid(),
                None,
                &mut name_length,
                None,
                &mut domain_length,
                &mut kind,
            )
        };
        ensure!(name_length > 0, "could not size account name");
        let mut name = vec![0; name_length as usize];
        let mut domain = vec![0; domain_length as usize];
        unsafe {
            LookupAccountSidW(
                PCWSTR::null(),
                self.sid(),
                Some(PWSTR(name.as_mut_ptr())),
                &mut name_length,
                Some(PWSTR(domain.as_mut_ptr())),
                &mut domain_length,
                &mut kind,
            )
        }?;
        let name = String::from_utf16(
            &name[..name
                .iter()
                .position(|unit| *unit == 0)
                .unwrap_or(name.len())],
        )?;
        let domain = String::from_utf16(
            &domain[..domain
                .iter()
                .position(|unit| *unit == 0)
                .unwrap_or(domain.len())],
        )?;
        let mut computer = [0u16; 256];
        let mut computer_length = computer.len() as u32;
        unsafe { GetComputerNameW(Some(PWSTR(computer.as_mut_ptr())), &mut computer_length) }?;
        let computer = String::from_utf16(&computer[..computer_length as usize])?;
        // Win32-OpenSSH matches local and Microsoft accounts without a domain.
        // AD accounts retain their SAM-compatible domain\user form.
        Ok(if domain.is_empty()
            || domain.eq_ignore_ascii_case(&computer)
            || domain.eq_ignore_ascii_case("MicrosoftAccount")
        {
            name
        } else {
            format!("{domain}\\{name}")
        }
        .to_lowercase())
    }
}
