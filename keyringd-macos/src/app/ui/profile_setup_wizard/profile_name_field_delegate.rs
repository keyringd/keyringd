use keyringd_ui_core::runtime::UiEventTarget;
use objc2::rc::Retained;
use objc2::runtime::NSObjectProtocol;
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send};
use objc2_app_kit::NSTextField;
use objc2_app_kit::{NSControlTextEditingDelegate, NSTextFieldDelegate};
use objc2_foundation::{NSNotification, NSObject};

use crate::app::ui::profile_setup_wizard::ProfileSetupEvent;

pub(super) struct ProfileNameFieldDelegateIvars {
    field: Retained<NSTextField>,
    events: UiEventTarget<ProfileSetupEvent>,
}

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "KeyringdProfileNameFieldDelegate"]
    #[ivars = ProfileNameFieldDelegateIvars]
    pub(super) struct ProfileNameFieldDelegate;

    unsafe impl NSObjectProtocol for ProfileNameFieldDelegate {}

    unsafe impl NSControlTextEditingDelegate for ProfileNameFieldDelegate {
        #[unsafe(method(controlTextDidChange:))]
        fn control_text_did_change(&self, _notification: &NSNotification) {
            self.ivars().events.send(ProfileSetupEvent::NameChanged(
                self.ivars().field.stringValue().to_string(),
            ));
        }
    }

    unsafe impl NSTextFieldDelegate for ProfileNameFieldDelegate {}
);

impl ProfileNameFieldDelegate {
    pub(super) fn new(
        mtm: MainThreadMarker,
        field: Retained<objc2_app_kit::NSTextField>,
        events: UiEventTarget<ProfileSetupEvent>,
    ) -> Retained<Self> {
        let allocated = mtm
            .alloc()
            .set_ivars(ProfileNameFieldDelegateIvars { field, events });
        unsafe { msg_send![super(allocated), init] }
    }
}
