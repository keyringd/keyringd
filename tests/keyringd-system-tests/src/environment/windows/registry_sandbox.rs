use anyhow::{Result, ensure};
use windows::Win32::Foundation::{ERROR_FILE_NOT_FOUND, ERROR_SUCCESS};
use windows::Win32::System::Registry::{
    HKEY_CURRENT_USER, RRF_RT_REG_SZ, RegDeleteTreeW, RegGetValueW,
};
use windows::core::{PCWSTR, w};

/// Owns only a UUID key in the UI-test namespace, never HKCU\Environment.
pub(crate) struct RegistrySandbox {
    key: String,
}

impl RegistrySandbox {
    pub(crate) fn new() -> Self {
        Self {
            key: format!(r"Software\keyringd-ui-tests\{}", uuid::Uuid::new_v4()),
        }
    }
    pub(crate) fn key(&self) -> &str {
        &self.key
    }
    pub(crate) fn read_agent(&self) -> Result<Option<String>> {
        let key: Vec<u16> = self.key.encode_utf16().chain(Some(0)).collect();
        let mut value = [0u16; 512];
        let mut size = std::mem::size_of_val(&value) as u32;
        let status = unsafe {
            RegGetValueW(
                HKEY_CURRENT_USER,
                PCWSTR(key.as_ptr()),
                w!("SSH_AUTH_SOCK"),
                RRF_RT_REG_SZ,
                None,
                Some(value.as_mut_ptr().cast()),
                Some(&mut size),
            )
        };
        if status == ERROR_FILE_NOT_FOUND {
            return Ok(None);
        }
        ensure!(
            status == ERROR_SUCCESS,
            "read sandbox SSH_AUTH_SOCK: {}",
            status.0
        );
        Ok(Some(String::from_utf16(&value[..size as usize / 2 - 1])?))
    }
}

impl Drop for RegistrySandbox {
    fn drop(&mut self) {
        let key: Vec<u16> = self.key.encode_utf16().chain(Some(0)).collect();
        unsafe {
            let _ = RegDeleteTreeW(HKEY_CURRENT_USER, PCWSTR(key.as_ptr()));
        }
    }
}
