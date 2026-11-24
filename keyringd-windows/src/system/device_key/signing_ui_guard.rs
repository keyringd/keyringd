use windows::Win32::Foundation::{HWND, POINT};
use windows::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromPoint,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetCursorPos, GetForegroundWindow, HWND_NOTOPMOST, HWND_TOPMOST, IsWindow, SW_HIDE, SW_SHOW,
    SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, SetForegroundWindow, SetWindowPos, ShowWindow,
};

pub(super) struct SigningUiGuard {
    owner: isize,
    previous_foreground: isize,
}

impl SigningUiGuard {
    pub(super) fn show(owner: isize) -> Self {
        let previous_foreground = unsafe { GetForegroundWindow().0 as isize };
        if owner != 0 {
            center_owner_on_cursor_monitor(owner);
            let window = HWND(owner as *mut core::ffi::c_void);
            unsafe {
                let _ = ShowWindow(window, SW_SHOW);
                let _ = SetWindowPos(
                    window,
                    Some(HWND_TOPMOST),
                    0,
                    0,
                    0,
                    0,
                    SWP_NOMOVE | SWP_NOSIZE,
                );
                if !SetForegroundWindow(window).as_bool() {
                    tracing::warn!("Windows refused to foreground the CNG owner window");
                }
            }
        }
        Self {
            owner,
            previous_foreground,
        }
    }
}

impl Drop for SigningUiGuard {
    fn drop(&mut self) {
        if self.owner == 0 {
            return;
        }
        let window = HWND(self.owner as *mut core::ffi::c_void);
        unsafe {
            let restore_previous = GetForegroundWindow() == window
                && self.previous_foreground != 0
                && self.previous_foreground != self.owner
                && IsWindow(Some(HWND(
                    self.previous_foreground as *mut core::ffi::c_void,
                )))
                .as_bool();
            let _ = SetWindowPos(
                window,
                Some(HWND_NOTOPMOST),
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE,
            );
            let _ = ShowWindow(window, SW_HIDE);
            if restore_previous {
                let _ =
                    SetForegroundWindow(HWND(self.previous_foreground as *mut core::ffi::c_void));
            }
        }
    }
}

fn center_owner_on_cursor_monitor(owner: isize) {
    if owner == 0 {
        return;
    }
    let mut cursor = POINT::default();
    if unsafe { GetCursorPos(&mut cursor) }.is_err() {
        return;
    }
    let monitor = unsafe { MonitorFromPoint(cursor, MONITOR_DEFAULTTONEAREST) };
    let mut info = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    if !unsafe { GetMonitorInfoW(monitor, &mut info) }.as_bool() {
        return;
    }
    let x = info.rcWork.left + (info.rcWork.right - info.rcWork.left) / 2;
    let y = info.rcWork.top + (info.rcWork.bottom - info.rcWork.top) / 2;
    unsafe {
        let _ = SetWindowPos(
            HWND(owner as *mut core::ffi::c_void),
            None,
            x,
            y,
            0,
            0,
            SWP_NOSIZE | SWP_NOZORDER,
        );
    }
}
