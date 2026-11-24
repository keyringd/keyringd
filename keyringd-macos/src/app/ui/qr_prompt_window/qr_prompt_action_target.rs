use super::QrPromptCommandHandler;
use crate::app::ui::AppShell;
use keyringd_ui_core::key_operation::KeyOperationEvent;
use keyringd_ui_core::runtime::UiEventSender;
use keyringd_ui_core::{
    ComponentModel,
    qr_prompt_window::{QrPromptEvent, QrPromptState},
};
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send};
use objc2_app_kit::NSWindow;
use objc2_foundation::NSObject;
use std::cell::RefCell;

pub(super) struct QrPromptActionTargetIvars {
    handler: QrPromptCommandHandler,
    state: RefCell<QrPromptState>,
}
define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "KeyringdQrPromptActionTarget"]
    #[ivars = QrPromptActionTargetIvars]
    pub(super) struct QrPromptActionTarget;

    impl QrPromptActionTarget {
        #[unsafe(method(cancelOperation:))]
        fn cancel_operation(&self, _sender: Option<&AnyObject>) {
            self.dispatch(QrPromptEvent::CancelRequested);
        }
        #[unsafe(method(useSecurityKey:))]
        fn use_security_key(&self, _sender: Option<&AnyObject>) {
            self.dispatch(QrPromptEvent::UseUsbRequested);
        }
    }
);

impl QrPromptActionTarget {
    fn dispatch(&self, event: QrPromptEvent) {
        let command = self.ivars().state.borrow_mut().update(event);
        if let Some(command) = command {
            self.ivars().handler.handle(command);
        }
    }
    pub(super) fn dismiss(&self) {
        self.dispatch(QrPromptEvent::Dismissed);
    }

    pub(super) fn usb_selected(&self) {
        self.dispatch(QrPromptEvent::UsbSelected);
    }

    pub(super) fn new(
        mtm: MainThreadMarker,
        events: UiEventSender<KeyOperationEvent>,
        prompt_id: keyringd_fido_client::InteractionId,
        window: Retained<NSWindow>,
        shell: Retained<AppShell>,
        parent: Option<Retained<NSWindow>>,
    ) -> Retained<Self> {
        let allocated = mtm.alloc().set_ivars(QrPromptActionTargetIvars {
            handler: QrPromptCommandHandler::new(events, prompt_id, window, shell, parent),
            state: RefCell::new(QrPromptState::default()),
        });
        unsafe { msg_send![super(allocated), init] }
    }
}
