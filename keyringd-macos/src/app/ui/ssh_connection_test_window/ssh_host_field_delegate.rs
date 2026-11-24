use keyringd_ui_core::runtime::UiEventSender;
use objc2::rc::Retained;
use objc2::runtime::NSObjectProtocol;
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send};
use objc2_app_kit::{NSControlTextEditingDelegate, NSTextField, NSTextFieldDelegate};
use objc2_foundation::{NSNotification, NSObject};

use super::SshConnectionTestEvent;

pub(super) struct SshHostFieldDelegateIvars {
    field: Retained<NSTextField>,
    events: UiEventSender<SshConnectionTestEvent>,
}

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "KeyringdSshHostFieldDelegate"]
    #[ivars = SshHostFieldDelegateIvars]
    pub(super) struct SshHostFieldDelegate;

    unsafe impl NSObjectProtocol for SshHostFieldDelegate {}

    unsafe impl NSControlTextEditingDelegate for SshHostFieldDelegate {
        #[unsafe(method(controlTextDidChange:))]
        fn control_text_did_change(&self, _notification: &NSNotification) {
            self.events().send(SshConnectionTestEvent::HostChanged(
                self.ivars().field.stringValue().to_string(),
            ));
        }
    }

    unsafe impl NSTextFieldDelegate for SshHostFieldDelegate {}
);

impl SshHostFieldDelegate {
    pub(super) fn new(
        mtm: MainThreadMarker,
        field: Retained<NSTextField>,
        events: UiEventSender<SshConnectionTestEvent>,
    ) -> Retained<Self> {
        let allocated = mtm
            .alloc()
            .set_ivars(SshHostFieldDelegateIvars { field, events });
        unsafe { msg_send![super(allocated), init] }
    }

    fn events(&self) -> &UiEventSender<SshConnectionTestEvent> {
        &self.ivars().events
    }
}
