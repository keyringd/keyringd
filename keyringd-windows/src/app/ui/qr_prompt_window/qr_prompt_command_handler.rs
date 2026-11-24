use keyringd_core::application::ActionId;
use std::sync::Arc;

use windows_reactor::{RenderCx, Updater};

use crate::app::AuthenticationPromptCommandHandler;
use crate::app::AuthenticationPromptWindows;
use keyringd_fido_client::HybridTransportUnavailableReason;
use keyringd_ui_core::qr_prompt_window::QrPromptCommand;
use keyringd_ui_core::qr_prompt_window::QrPromptEvent;
use keyringd_ui_core::qr_prompt_window::QrPromptState;
use keyringd_ui_core::runtime::UiEventReceiver;
use std::cell::Cell;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{PostMessageW, WM_CLOSE};

#[derive(Clone)]
pub(super) struct QrPromptCommandHandler {
    flow: ActionId,
    prompts: AuthenticationPromptCommandHandler,
    session: AuthenticationPromptWindows,
    request_id: keyringd_fido_client::InteractionId,
    completed: Arc<AtomicBool>,
    window_handle: Rc<Cell<usize>>,
    set_state: Updater<QrPromptState>,
}

impl QrPromptCommandHandler {
    pub(super) fn new(
        flow: ActionId,
        prompts: AuthenticationPromptCommandHandler,
        session: AuthenticationPromptWindows,
        request_id: keyringd_fido_client::InteractionId,
        completed: Arc<AtomicBool>,
        window_handle: Rc<Cell<usize>>,
        set_state: Updater<QrPromptState>,
    ) -> Self {
        Self {
            flow,
            prompts,
            session,
            request_id,
            completed,
            window_handle,
            set_state,
        }
    }

    pub(super) fn use_session_status(
        &self,
        cx: &mut RenderCx,
    ) -> (bool, Option<HybridTransportUnavailableReason>) {
        let mut changes = self.session.subscribe();
        let initial = *changes.borrow_and_update();
        let (state, set_state) = cx.use_async_state(initial);
        cx.use_effect_with_cleanup((), {
            let runtime = self.prompts.runtime.clone();
            move || {
                let (initial, mut changes) = UiEventReceiver::from_tokio_watch(&runtime, changes);
                set_state.call(initial);
                let task = runtime.spawn(async move {
                    while let Some(state) = changes.receive().await {
                        set_state.call(state);
                    }
                });
                Some(move || task.abort())
            }
        });
        (
            !state.is_active(self.request_id),
            state.hybrid_unavailable(self.request_id),
        )
    }

    pub(super) fn handle(&self, command: QrPromptCommand) {
        match command {
            QrPromptCommand::UseUsb => {
                self.send(keyringd_ui_core::key_operation::KeyOperationEvent::HybridQrUseSecurityKeyRequested { prompt_id: self.request_id });
                self.completed.store(true, Ordering::Release);
                crate::app::ui::reduce(&self.set_state, QrPromptEvent::UsbSelected);
            }
            QrPromptCommand::Cancel => {
                self.completed.store(true, Ordering::Release);
                self.send(
                    keyringd_ui_core::key_operation::KeyOperationEvent::HybridQrCancelled {
                        prompt_id: self.request_id,
                    },
                );
            }
            QrPromptCommand::Close => {
                self.completed.store(true, Ordering::Release);
                let handle = self.window_handle.get();
                if handle != 0 {
                    unsafe {
                        let _ = PostMessageW(
                            Some(HWND(handle as *mut _)),
                            WM_CLOSE,
                            WPARAM(0),
                            LPARAM(0),
                        );
                    }
                }
            }
        }
    }

    pub(super) fn on_unmount(&self) {
        if !self.completed.load(Ordering::Acquire) {
            self.send(
                keyringd_ui_core::key_operation::KeyOperationEvent::HybridQrCancelled {
                    prompt_id: self.request_id,
                },
            );
        }
        self.session.qr_closed(self.request_id);
    }

    fn send(&self, event: keyringd_ui_core::key_operation::KeyOperationEvent) {
        if let Some(events) = self.prompts.coordinator.event_sender(self.flow) {
            events.send(event);
        }
    }
}
