use anyhow::Result;
use windows::Win32::{
    Foundation::{ERROR_SUCCESS, GetLastError, HWND, SetLastError},
    UI::WindowsAndMessaging::{
        GW_OWNER, GWL_EXSTYLE, GetWindow, GetWindowLongPtrW, SetWindowLongPtrW, WS_EX_APPWINDOW,
        WS_EX_TOOLWINDOW,
    },
};

/// Temporarily makes owned and tool windows eligible for Windows Graphics Capture.
///
/// The Server 2025 (26100) Actions runner rejects owned and tool windows
/// (including native popup menus) without WS_EX_APPWINDOW with E_INVALIDARG,
/// while the local Windows 11 (26200) host accepts them. Keep this test-driver
/// adjustment alive until capture ends, and restore the application's style
/// on both success and failure. No owner or production window settings change.
pub(crate) struct WindowCaptureStyle {
    window: HWND,
    original: Option<isize>,
}

impl WindowCaptureStyle {
    pub(crate) fn enter(window: HWND) -> Result<Self> {
        let mut guard = Self {
            window,
            original: None,
        };
        let style = unsafe { GetWindowLongPtrW(window, GWL_EXSTYLE) };
        let needs_app_window = unsafe { GetWindow(window, GW_OWNER) }.is_ok()
            || style & WS_EX_TOOLWINDOW.0 as isize != 0;
        if needs_app_window && style & WS_EX_APPWINDOW.0 as isize == 0 {
            unsafe {
                SetLastError(ERROR_SUCCESS);
                let previous =
                    SetWindowLongPtrW(window, GWL_EXSTYLE, style | WS_EX_APPWINDOW.0 as isize);
                // A zero previous style is valid; only a nonzero last error fails.
                if previous == 0 {
                    GetLastError().ok()?;
                }
                guard.original = Some(previous);
            }
        }
        Ok(guard)
    }
}

impl Drop for WindowCaptureStyle {
    fn drop(&mut self) {
        if let Some(original) = self.original {
            unsafe {
                SetWindowLongPtrW(self.window, GWL_EXSTYLE, original);
            }
        }
    }
}
