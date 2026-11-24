use windows::Win32::Foundation::{ERROR_FILE_NOT_FOUND, ERROR_SUCCESS, LPARAM, WPARAM};
use windows::Win32::System::Registry::{
    HKEY, HKEY_CURRENT_USER, KEY_SET_VALUE, REG_OPTION_NON_VOLATILE, REG_SZ, RRF_RT_REG_SZ,
    RegCloseKey, RegCreateKeyExW, RegDeleteKeyValueW, RegGetValueW, RegSetKeyValueW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    HWND_BROADCAST, SMTO_ABORTIFHUNG, SendMessageTimeoutW, WM_SETTINGCHANGE,
};
use windows::core::{PCWSTR, w};

use super::agent_pipe_name;
pub(crate) const GIT_FOR_WINDOWS_SSH_SETTING: &str =
    "export SSH_AUTH_SOCK=\"$(cygpath -u \"$LOCALAPPDATA\")/keyringd/agent.sock\"";

pub(crate) fn configure() -> std::io::Result<()> {
    write_agent(w!("Environment"), &agent_pipe_name()?)?;
    broadcast_environment_change();
    Ok(())
}

pub(crate) fn has_managed_agent_configuration() -> std::io::Result<bool> {
    Ok(read_agent(w!("Environment"))?.as_deref() == Some(agent_pipe_name()?.as_str()))
}

pub(crate) fn unconfigure() -> std::io::Result<()> {
    remove_agent(w!("Environment"), &agent_pipe_name()?)?;
    broadcast_environment_change();
    Ok(())
}

#[cfg(feature = "test-support")]
pub(crate) fn has_managed_agent_configuration_at(key: &str, pipe: &str) -> std::io::Result<bool> {
    let key: Vec<u16> = key.encode_utf16().chain(Some(0)).collect();
    Ok(read_agent(PCWSTR(key.as_ptr()))?.as_deref() == Some(pipe))
}

#[cfg(feature = "test-support")]
pub(crate) fn unconfigure_test(key: &str, pipe: &str) -> std::io::Result<()> {
    let key: Vec<u16> = key.encode_utf16().chain(Some(0)).collect();
    remove_agent(PCWSTR(key.as_ptr()), pipe)
}

fn read_agent(subkey: PCWSTR) -> std::io::Result<Option<String>> {
    let mut value = vec![0u16; 32768];
    let mut bytes = (value.len() * size_of::<u16>()) as u32;
    let status = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            subkey,
            w!("SSH_AUTH_SOCK"),
            RRF_RT_REG_SZ,
            None,
            Some(value.as_mut_ptr().cast()),
            Some(&mut bytes),
        )
    };
    if status == ERROR_FILE_NOT_FOUND {
        return Ok(None);
    }
    if status != ERROR_SUCCESS {
        return Err(std::io::Error::from_raw_os_error(status.0 as i32));
    }
    let end = value
        .iter()
        .position(|unit| *unit == 0)
        .unwrap_or(value.len());
    String::from_utf16(&value[..end])
        .map(Some)
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))
}

fn remove_agent(subkey: PCWSTR, pipe: &str) -> std::io::Result<()> {
    if read_agent(subkey)?.as_deref() != Some(pipe) {
        return Ok(());
    }
    let status = unsafe { RegDeleteKeyValueW(HKEY_CURRENT_USER, subkey, w!("SSH_AUTH_SOCK")) };
    if status != ERROR_SUCCESS && status != ERROR_FILE_NOT_FOUND {
        return Err(std::io::Error::from_raw_os_error(status.0 as i32));
    }
    Ok(())
}

#[cfg(feature = "test-support")]
pub(crate) fn configure_test(key: &str, pipe: &str) -> std::io::Result<()> {
    let key: Vec<u16> = key.encode_utf16().chain(Some(0)).collect();
    write_agent(PCWSTR(key.as_ptr()), pipe)
}

fn write_agent(subkey: PCWSTR, pipe: &str) -> std::io::Result<()> {
    let mut key = HKEY::default();
    let status = unsafe {
        RegCreateKeyExW(
            HKEY_CURRENT_USER,
            subkey,
            None,
            None,
            REG_OPTION_NON_VOLATILE,
            KEY_SET_VALUE,
            None,
            &mut key,
            None,
        )
    };
    if status != ERROR_SUCCESS {
        return Err(std::io::Error::from_raw_os_error(status.0 as i32));
    }
    let value: Vec<u16> = pipe.encode_utf16().chain(Some(0)).collect();
    let status = unsafe {
        RegSetKeyValueW(
            key,
            None,
            w!("SSH_AUTH_SOCK"),
            REG_SZ.0,
            Some(value.as_ptr().cast()),
            (value.len() * size_of::<u16>()) as u32,
        )
    };
    unsafe {
        let _ = RegCloseKey(key);
    }
    if status != ERROR_SUCCESS {
        return Err(std::io::Error::from_raw_os_error(status.0 as i32));
    }
    Ok(())
}

fn broadcast_environment_change() {
    unsafe {
        let _ = SendMessageTimeoutW(
            HWND_BROADCAST,
            WM_SETTINGCHANGE,
            WPARAM(0),
            LPARAM(w!("Environment").as_ptr() as isize),
            SMTO_ABORTIFHUNG,
            5_000,
            None,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    keyringd_testkit::scenario! {
        group "read_agent" {
            case "Removal preserves another agent and is repeatable" {
                verify "preserve another agent's setting across repeated removal attempts" {
                    let key = format!(
                        "Software\\keyringd-tests\\ssh-environment-{}-{}",
                        std::process::id(),
                        std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_nanos()
                    );
                    let wide: Vec<u16> = key.encode_utf16().chain(Some(0)).collect();
                    let subkey = PCWSTR(wide.as_ptr());
                    let pipe = agent_pipe_name().unwrap();
                    let result = std::panic::catch_unwind(|| {
                        assert_eq!(read_agent(subkey).unwrap(), None);
                        write_agent(subkey, &pipe).unwrap();
                        assert_eq!(
                            read_agent(subkey).unwrap().as_deref(),
                            Some(pipe.as_str())
                        );
                        remove_agent(subkey, &pipe).unwrap();
                        remove_agent(subkey, &pipe).unwrap();
                        assert_eq!(read_agent(subkey).unwrap(), None);
                        write_agent(subkey, "another-agent").unwrap();
                        remove_agent(subkey, &pipe).unwrap();
                        assert_eq!(
                            read_agent(subkey).unwrap().as_deref(),
                            Some("another-agent")
                        );
                    });
                    unsafe {
                        let _ = windows::Win32::System::Registry::RegDeleteTreeW(HKEY_CURRENT_USER, subkey);
                    }
                    if let Err(error) = result {
                        std::panic::resume_unwind(error);
                    }
                }
            }
        }
    }
}
