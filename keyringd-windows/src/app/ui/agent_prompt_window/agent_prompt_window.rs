use std::sync::Arc;

use crate::app::ui::agent_prompt_window::AgentPromptThumbnail;
use crate::app::ui::agent_prompt_window::AgentPromptWindowActivationSession;

use std::sync::mpsc;

use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, HWND_NOTOPMOST, HWND_TOPMOST,
    MSG, RegisterClassW, SW_HIDE, SW_SHOW, SWP_NOMOVE, SWP_NOSIZE, SetForegroundWindow,
    SetWindowPos, ShowWindow, TranslateMessage, WM_DWMSENDICONICTHUMBNAIL, WNDCLASSW, WS_CAPTION,
    WS_EX_APPWINDOW, WS_EX_LAYERED, WS_SYSMENU,
};
use windows::core::w;

use crate::app::ui::util::{center_on_cursor_monitor, load_application_icon};

const WINDOW_WIDTH: i32 = 1;
const WINDOW_HEIGHT: i32 = 1;

pub(crate) struct AgentPromptWindow {
    pub(super) handle: usize,
}

impl AgentPromptWindow {
    pub(crate) fn start() -> std::io::Result<Arc<Self>> {
        let (sender, receiver) = mpsc::sync_channel(1);
        std::thread::Builder::new()
            .name("keyringd-agent-prompt".to_string())
            .spawn(move || Self::run_message_loop(sender))?;
        receiver
            .recv()
            .map_err(|_| std::io::Error::other("agent prompt window thread stopped"))?
            .map(|handle| Arc::new(Self { handle }))
    }

    pub(crate) fn hwnd(&self) -> HWND {
        HWND(self.handle as *mut core::ffi::c_void)
    }

    pub(crate) fn show(&self) {
        let window = self.hwnd();
        center_on_cursor_monitor(window);
        unsafe {
            let _ = ShowWindow(window, SW_SHOW);
            if let Err(error) = SetWindowPos(
                window,
                Some(HWND_TOPMOST),
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE,
            ) {
                tracing::warn!(%error, "failed to keep the agent prompt window in front");
            }
        }
        if !unsafe { SetForegroundWindow(window) }.as_bool() {
            tracing::warn!("Windows refused to foreground the agent prompt window");
        }
    }

    pub(crate) fn hide(&self) {
        let window = self.hwnd();
        unsafe {
            if let Err(error) = SetWindowPos(
                window,
                Some(HWND_NOTOPMOST),
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE,
            ) {
                tracing::warn!(%error, "failed to clear the agent prompt window topmost state");
            }
            let _ = ShowWindow(window, SW_HIDE);
        }
    }

    pub(crate) fn activate(&self) -> AgentPromptWindowActivationSession {
        self.show();
        AgentPromptWindowActivationSession::new(self.handle)
    }

    fn run_message_loop(ready: mpsc::SyncSender<std::io::Result<usize>>) {
        let result = Self::create();
        let window = match result {
            Ok(window) => window,
            Err(error) => {
                keyringd_core::util::FailureDiagnostics::record_error(
                    "Agent prompt UI thread",
                    &error,
                );
                let _ = ready.send(Err(error));
                return;
            }
        };
        if ready.send(Ok(window.0 as usize)).is_err() {
            return;
        }

        let mut message = MSG::default();
        while unsafe { GetMessageW(&mut message, None, 0, 0) }.as_bool() {
            unsafe {
                let _ = TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }
    }

    fn create() -> std::io::Result<HWND> {
        let instance = unsafe { GetModuleHandleW(None) }.map_err(std::io::Error::other)?;
        let class = WNDCLASSW {
            hInstance: instance.into(),
            hIcon: load_application_icon().unwrap_or_default(),
            lpszClassName: w!("keyringd-authentication-owner"),
            lpfnWndProc: Some(Self::window_proc),
            ..Default::default()
        };
        if unsafe { RegisterClassW(&class) } == 0 {
            return Err(std::io::Error::last_os_error());
        }
        let window = unsafe {
            CreateWindowExW(
                WS_EX_APPWINDOW | WS_EX_LAYERED,
                class.lpszClassName,
                w!("Keyringd authentication"),
                WS_CAPTION | WS_SYSMENU,
                0,
                0,
                WINDOW_WIDTH,
                WINDOW_HEIGHT,
                None,
                None,
                Some(instance.into()),
                None,
            )
        }
        .map_err(std::io::Error::other)?;
        if let Err(error) = AgentPromptThumbnail::configure(window) {
            tracing::warn!(%error, "failed to configure the authentication taskbar window");
        }
        center_on_cursor_monitor(window);
        Ok(window)
    }

    unsafe extern "system" fn window_proc(
        window: HWND,
        message: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        if message == WM_DWMSENDICONICTHUMBNAIL {
            let width = ((lparam.0 as u32 >> 16) & 0xffff).max(1);
            let height = (lparam.0 as u32 & 0xffff).max(1);
            if let Err(error) = AgentPromptThumbnail::render(window, width, height) {
                tracing::warn!(%error, "failed to render the authentication taskbar thumbnail");
            }
            return LRESULT(0);
        }
        unsafe { DefWindowProcW(window, message, wparam, lparam) }
    }
}

impl crate::system::webauthn::WebAuthnWindow for AgentPromptWindow {
    type Session = AgentPromptWindowActivationSession;

    fn handle(&self) -> HWND {
        self.hwnd()
    }

    fn activate(&self) -> Self::Session {
        AgentPromptWindow::activate(self)
    }
}
