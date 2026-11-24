use super::TrayCommandHandler;
use crate::app::ui::ComponentModel;
use keyringd_core::model::Profile;
use keyringd_ui_core::app_menu::AppMenuEvent;
use keyringd_ui_core::app_menu::AppMenuLifecycleEvent;
use keyringd_ui_core::app_menu::AppMenuState;
use std::os::windows::io::{AsRawHandle, OwnedHandle};
use std::sync::mpsc;
use tokio::sync::mpsc::UnboundedSender;
use tray_icon::menu::MenuEvent;
use windows::Win32::Foundation::{HANDLE, WAIT_FAILED};
use windows::Win32::System::Threading::INFINITE;
use windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, MSG, MWMO_INPUTAVAILABLE, MsgWaitForMultipleObjectsEx, PM_REMOVE,
    PeekMessageW, QS_ALLINPUT, TranslateMessage,
};

pub(super) struct TrayComponent;

impl TrayComponent {
    pub(super) fn run(
        profiles: Vec<Profile>,
        updates: mpsc::Receiver<AppMenuLifecycleEvent>,
        events: UnboundedSender<AppMenuEvent>,
        wake: &OwnedHandle,
    ) {
        let mut state = AppMenuState::default();
        let mut handler = TrayCommandHandler::new(events);
        if !Self::dispatch(
            &mut state,
            &mut handler,
            AppMenuLifecycleEvent::Mounted(profiles),
        ) {
            return;
        }
        let handles = [HANDLE(wake.as_raw_handle())];
        loop {
            if let Some(update) = updates.try_iter().last()
                && !Self::dispatch(&mut state, &mut handler, update)
            {
                return;
            }
            Self::pump_windows_messages();
            while let Ok(event) = MenuEvent::receiver().try_recv() {
                if let Some(action) = handler.action(event.id().as_ref())
                    && !Self::dispatch(
                        &mut state,
                        &mut handler,
                        AppMenuLifecycleEvent::ActionRequested(action),
                    )
                {
                    return;
                }
            }
            let ready = unsafe {
                MsgWaitForMultipleObjectsEx(
                    Some(&handles),
                    INFINITE,
                    QS_ALLINPUT,
                    MWMO_INPUTAVAILABLE,
                )
            };
            if ready == WAIT_FAILED {
                tracing::error!(error = %std::io::Error::last_os_error(), "tray wait failed");
                return;
            }
        }
    }

    fn dispatch(
        state: &mut AppMenuState,
        handler: &mut TrayCommandHandler,
        event: AppMenuLifecycleEvent,
    ) -> bool {
        let delivered = state
            .update(event)
            .is_none_or(|command| handler.handle(command));
        delivered && !state.is_stopped()
    }

    fn pump_windows_messages() {
        unsafe {
            let mut message = MSG::default();
            while PeekMessageW(&mut message, None, 0, 0, PM_REMOVE).as_bool() {
                let _ = TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }
    }
}
