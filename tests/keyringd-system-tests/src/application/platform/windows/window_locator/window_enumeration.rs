use super::NamedWindowLookup;
use super::OwnedWindowLookup;
use super::WindowDescriptionLookup;
use super::WindowLookup;

use anyhow::Result;
use windows::Win32::Foundation::{HWND, LPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetWindowTextW, GetWindowThreadProcessId, IsWindowVisible,
};
use windows::core::BOOL;

pub(crate) fn visible_windows(pid: u32) -> Result<Vec<HWND>> {
    windows(pid, true)
}

pub(crate) fn windows(pid: u32, visible_only: bool) -> Result<Vec<HWND>> {
    unsafe extern "system" fn inspect(window: HWND, parameter: LPARAM) -> BOOL {
        let lookup = unsafe { &mut *(parameter.0 as *mut WindowLookup) };
        let mut owner = 0;
        unsafe {
            GetWindowThreadProcessId(window, Some(&mut owner));
        }
        if owner == lookup.pid
            && (!lookup.visible_only || unsafe { IsWindowVisible(window) }.as_bool())
        {
            lookup.windows.push(window);
        }
        true.into()
    }
    let mut lookup = WindowLookup {
        pid,
        windows: Vec::new(),
        visible_only,
    };
    unsafe {
        EnumWindows(Some(inspect), LPARAM(&mut lookup as *mut _ as isize))?;
    }
    Ok(lookup.windows)
}

pub(crate) fn named_window(pid: u32, title: &str) -> Result<Option<HWND>> {
    Ok(visible_windows(pid)?.into_iter().find(|window| {
        let mut caption = [0u16; 512];
        let length = unsafe { GetWindowTextW(*window, &mut caption) };
        String::from_utf16_lossy(&caption[..length as usize]) == title
    }))
}

/// Only OS windows whose owner chain leads to this API call's owner HWND.
pub(crate) fn owned_windows(owner: usize) -> Result<Vec<HWND>> {
    use windows::Win32::UI::WindowsAndMessaging::{GW_OWNER, GetWindow};
    unsafe extern "system" fn inspect(window: HWND, parameter: LPARAM) -> BOOL {
        let lookup = unsafe { &mut *(parameter.0 as *mut OwnedWindowLookup) };
        if unsafe { IsWindowVisible(window) }.as_bool() {
            let mut current = window;
            for _ in 0..16 {
                let Ok(parent) = (unsafe { GetWindow(current, GW_OWNER) }) else {
                    break;
                };
                if parent.0 as usize == lookup.owner {
                    lookup.windows.push(window);
                    break;
                }
                if parent.0.is_null() {
                    break;
                }
                current = parent;
            }
        }
        true.into()
    }
    let mut lookup = OwnedWindowLookup {
        owner,
        windows: Vec::new(),
    };
    unsafe {
        EnumWindows(Some(inspect), LPARAM(&mut lookup as *mut _ as isize))?;
    }
    Ok(lookup.windows)
}

pub(crate) fn visible_window_descriptions() -> Result<Vec<String>> {
    unsafe extern "system" fn inspect(window: HWND, parameter: LPARAM) -> BOOL {
        if unsafe { IsWindowVisible(window) }.as_bool() {
            let mut caption = [0u16; 512];
            let length = unsafe { GetWindowTextW(window, &mut caption) };
            if length > 0 {
                let mut pid = 0;
                unsafe { GetWindowThreadProcessId(window, Some(&mut pid)) };
                let lookup = unsafe { &mut *(parameter.0 as *mut WindowDescriptionLookup) };
                lookup.0.push(format!(
                    "PID {pid} HWND {}: {}",
                    window.0 as usize,
                    String::from_utf16_lossy(&caption[..length as usize]),
                ));
            }
        }
        true.into()
    }
    let mut lookup = WindowDescriptionLookup(Vec::new());
    unsafe { EnumWindows(Some(inspect), LPARAM(&mut lookup as *mut _ as isize))? };
    Ok(lookup.0)
}

pub(crate) fn visible_windows_named(title: &str) -> Result<Vec<HWND>> {
    unsafe extern "system" fn inspect(window: HWND, parameter: LPARAM) -> BOOL {
        let lookup = unsafe { &mut *(parameter.0 as *mut NamedWindowLookup) };
        if unsafe { IsWindowVisible(window) }.as_bool() {
            let mut caption = [0u16; 512];
            let length = unsafe { GetWindowTextW(window, &mut caption) };
            if String::from_utf16_lossy(&caption[..length as usize]) == lookup.title {
                lookup.windows.push(window);
            }
        }
        true.into()
    }
    let mut lookup = NamedWindowLookup {
        title: title.to_owned(),
        windows: Vec::new(),
    };
    unsafe { EnumWindows(Some(inspect), LPARAM(&mut lookup as *mut _ as isize))? };
    Ok(lookup.windows)
}
