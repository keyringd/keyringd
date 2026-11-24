use windows::Win32::{
    Foundation::HWND,
    System::Console::GetConsoleWindow,
    UI::WindowsAndMessaging::{IsIconic, IsWindowVisible, SW_MINIMIZE, SW_RESTORE, ShowWindow},
};

/// Keeps the caller's console from covering UI fixtures; restores it on every exit.
pub(crate) struct ConsoleGuard {
    window: HWND,
    restore: bool,
}
impl ConsoleGuard {
    pub fn minimize() -> Self {
        unsafe {
            let window = GetConsoleWindow();
            let restore = !window.0.is_null()
                && IsWindowVisible(window).as_bool()
                && !IsIconic(window).as_bool();
            if restore {
                let _ = ShowWindow(window, SW_MINIMIZE);
            }
            Self { window, restore }
        }
    }
}
impl Drop for ConsoleGuard {
    fn drop(&mut self) {
        if self.restore {
            unsafe {
                let _ = ShowWindow(self.window, SW_RESTORE);
            }
        }
    }
}
