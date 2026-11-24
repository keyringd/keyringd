use std::collections::HashSet;

use windows::Win32::Foundation::{HWND, LPARAM};
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::WindowsAndMessaging::{
    EnumThreadWindows, GA_ROOT, GetAncestor, GetClassNameW, IsWindow,
};
use windows::core::BOOL;

const REACTOR_WINDOW_CLASS: &str = "WinUIDesktopWin32WindowClass";

/// Locates the native top-level window just created by `windows-reactor`.
///
/// `windows-reactor::WindowHandle` does not currently expose its HWND. This isolates the
/// Win32 workaround until the library provides a native operation. Reactor windows are
/// identified by thread-local creation and WinUI's top-level window class; visible titles
/// are deliberately not used as identifiers.
pub(crate) struct CreatedReactorWindowLocator {
    existing_windows: HashSet<usize>,
}

impl CreatedReactorWindowLocator {
    pub(crate) fn begin() -> std::io::Result<Self> {
        Ok(Self {
            existing_windows: Self::thread_windows()?,
        })
    }

    pub(crate) fn finish(self) -> std::io::Result<HWND> {
        let created = Self::thread_windows()?
            .difference(&self.existing_windows)
            .copied()
            .collect::<Vec<_>>();
        let matching = created
            .iter()
            .copied()
            .filter(|value| Self::is_reactor_window(Self::hwnd(*value)))
            .collect::<Vec<_>>();

        match matching.as_slice() {
            [window] => Ok(Self::hwnd(*window)),
            _ => Err(std::io::Error::other(format!(
                "expected one new {REACTOR_WINDOW_CLASS} window, found {}; created windows: {}",
                matching.len(),
                created
                    .iter()
                    .map(|value| Self::describe(Self::hwnd(*value)))
                    .collect::<Vec<_>>()
                    .join(", ")
            ))),
        }
    }

    fn thread_windows() -> std::io::Result<HashSet<usize>> {
        let mut windows = HashSet::new();
        let result = unsafe {
            EnumThreadWindows(
                GetCurrentThreadId(),
                Some(Self::collect),
                LPARAM(std::ptr::from_mut(&mut windows) as isize),
            )
        };
        if !result.as_bool() {
            return Err(std::io::Error::last_os_error());
        }
        Ok(windows)
    }

    fn is_reactor_window(window: HWND) -> bool {
        unsafe { IsWindow(Some(window)) }.as_bool()
            && unsafe { GetAncestor(window, GA_ROOT) } == window
            && Self::class_name(window) == REACTOR_WINDOW_CLASS
    }

    fn describe(window: HWND) -> String {
        format!("hwnd={:?} class={:?}", window.0, Self::class_name(window))
    }

    fn class_name(window: HWND) -> String {
        let mut buffer = [0_u16; 256];
        let length = unsafe { GetClassNameW(window, &mut buffer) };
        String::from_utf16_lossy(&buffer[..length.max(0) as usize])
    }

    fn hwnd(value: usize) -> HWND {
        HWND(value as *mut core::ffi::c_void)
    }

    unsafe extern "system" fn collect(window: HWND, context: LPARAM) -> BOOL {
        let windows = unsafe { &mut *(context.0 as *mut HashSet<usize>) };
        windows.insert(window.0 as usize);
        true.into()
    }
}
