use crate::app::WindowsApplication;
use keyringd_ui_core::profile_setup_window::{ProfileSetupWindowEvent, ProfileSetupWindowState};
use keyringd_ui_core::text as ui_text;

use keyringd_ui_core::runtime::UiEventTarget;

use crate::app::ui::util::{CreatedReactorWindowLocator, center_on_cursor_monitor};

use windows::Win32::Foundation::HWND;

use windows_reactor::{Backdrop, Element, ReactorWindow, RenderCx};

use super::ProfileSetupWindowCommandHandler;
use crate::app::ui::profile_setup_wizard::ProfileSetupWizardView;
use std::sync::Arc;
use windows_reactor::Updater;

use crate::app::ui::util::ModalWindowLifecycle;
use std::cell::Cell;
use std::rc::Rc;

pub(crate) struct ProfileSetupWindow {
    coordinator: Arc<keyringd_ui_core::key_operation::KeyOperationCoordinator>,
    application: Arc<WindowsApplication>,
    runtime: tokio::runtime::Handle,
    parent: HWND,
    window_handle: Rc<Cell<usize>>,
}

impl ProfileSetupWindow {
    pub(crate) fn new(
        application: Arc<WindowsApplication>,
        runtime: tokio::runtime::Handle,
        parent: usize,
        coordinator: Arc<keyringd_ui_core::key_operation::KeyOperationCoordinator>,
    ) -> Self {
        Self {
            coordinator,
            application,
            runtime,
            parent: HWND(parent as *mut core::ffi::c_void),
            window_handle: Rc::new(Cell::new(0)),
        }
    }

    pub(crate) fn open(self) -> anyhow::Result<()> {
        let parent = self.parent;
        let window_handle = self.window_handle.clone();
        let created_window = CreatedReactorWindowLocator::begin()?;
        ReactorWindow::new()
            .title(ui_text::PROFILE_SETUP_WINDOW_TITLE)
            .inner_size(crate::app::ui::PASSKEY_DIALOG_WIDTH, 500.0)
            .backdrop(Backdrop::Mica)
            .render(move |cx| self.render(cx))?;

        let child = created_window.finish()?;
        center_on_cursor_monitor(child);
        crate::app::ui::util::apply_application_icon(child)?;
        window_handle.set(child.0 as usize);
        ModalWindowLifecycle::attach(child, parent)?;
        Ok(())
    }

    fn render(&self, cx: &mut RenderCx) -> Element {
        let (state, set_state) = cx.use_reducer(ProfileSetupWindowState::default());
        let events = crate::app::ui::util::UiEventSink::use_in(cx);
        let handler = ProfileSetupWindowCommandHandler::use_in(
            cx,
            self.application.clone(),
            self.runtime.clone(),
            self.window_handle.clone(),
            events.sender(),
            self.coordinator.clone(),
        );
        let operation_window = self.window_handle.clone();
        events.bind(UiEventTarget::new(move |event| {
            Self::dispatch(&set_state, &handler, event, operation_window.get())
        }));
        let height = if state.is_running() || state.advanced_settings() {
            690
        } else {
            500
        };
        Self::resize(cx, self.window_handle.clone(), height);
        let interaction = state.interaction_presentation();
        let source_pin = state.pin_request_id();
        let view_events = events.target().map(ProfileSetupWindowEvent::from);
        ProfileSetupWizardView::render(cx, &state, &interaction, source_pin, view_events)
    }

    fn dispatch(
        set_state: &Updater<ProfileSetupWindowState>,
        handler: &ProfileSetupWindowCommandHandler,
        event: ProfileSetupWindowEvent,
        window: usize,
    ) {
        let (state, command) = crate::app::ui::reduce(set_state, event);
        Self::set_native_close_enabled(window, !state.is_running());
        if let Some(command) = command
            && let Some(event) = handler.handle(command)
        {
            Self::dispatch(set_state, handler, event, window);
        }
    }
    fn set_native_close_enabled(value: usize, enabled: bool) {
        use windows::Win32::UI::WindowsAndMessaging::{
            DrawMenuBar, EnableMenuItem, GetSystemMenu, MF_BYCOMMAND, MF_ENABLED, MF_GRAYED,
            SC_CLOSE,
        };
        if value == 0 {
            return;
        }
        unsafe {
            let window = HWND(value as *mut _);
            let menu = GetSystemMenu(window, false);
            if !menu.0.is_null() {
                let _ = EnableMenuItem(
                    menu,
                    SC_CLOSE,
                    MF_BYCOMMAND | if enabled { MF_ENABLED } else { MF_GRAYED },
                );
                let _ = DrawMenuBar(window);
            }
        }
    }

    fn resize(cx: &mut RenderCx, window_handle: Rc<Cell<usize>>, height: i32) {
        cx.use_effect(height, move || {
            use windows::Win32::Foundation::RECT;
            use windows::Win32::UI::HiDpi::{AdjustWindowRectExForDpi, GetDpiForWindow};
            use windows::Win32::UI::WindowsAndMessaging::{
                GWL_EXSTYLE, GWL_STYLE, GetClientRect, GetWindowLongW, SWP_NOACTIVATE, SWP_NOMOVE,
                SWP_NOZORDER, SetWindowPos, WINDOW_EX_STYLE, WINDOW_STYLE,
            };
            let window = HWND(window_handle.get() as *mut _);
            if window.0.is_null() {
                return;
            }
            unsafe {
                let dpi = GetDpiForWindow(window);
                let mut current = RECT::default();
                if GetClientRect(window, &mut current).is_err() {
                    return;
                }
                let mut bounds = RECT {
                    left: 0,
                    top: 0,
                    right: ((crate::app::ui::PASSKEY_DIALOG_WIDTH * f64::from(dpi) / 96.0).round()
                        as i32)
                        .max(current.right - current.left),
                    bottom: (height * dpi as i32 / 96).max(current.bottom - current.top),
                };
                if AdjustWindowRectExForDpi(
                    &mut bounds,
                    WINDOW_STYLE(GetWindowLongW(window, GWL_STYLE) as u32),
                    false,
                    WINDOW_EX_STYLE(GetWindowLongW(window, GWL_EXSTYLE) as u32),
                    dpi,
                )
                .is_ok()
                {
                    let _ = SetWindowPos(
                        window,
                        None,
                        0,
                        0,
                        bounds.right - bounds.left,
                        bounds.bottom - bounds.top,
                        SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE,
                    );
                }
            }
        });
    }

    pub(super) fn close(value: usize) {
        use windows::Win32::Foundation::{LPARAM, WPARAM};
        use windows::Win32::UI::WindowsAndMessaging::{PostMessageW, WM_CLOSE};

        if value != 0 {
            unsafe {
                let _ = PostMessageW(Some(HWND(value as *mut _)), WM_CLOSE, WPARAM(0), LPARAM(0));
            }
        }
    }
}
