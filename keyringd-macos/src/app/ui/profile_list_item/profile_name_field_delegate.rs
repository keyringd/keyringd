use keyringd_core::model::ProfileId;
use keyringd_ui_core::runtime::UiEventTarget;
use objc2::rc::Retained;
use objc2::runtime::{NSObjectProtocol, Sel};
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send, sel};
use objc2_app_kit::NSTextField;
use objc2_app_kit::{NSControl, NSControlTextEditingDelegate, NSTextFieldDelegate, NSTextView};
use objc2_foundation::{NSNotification, NSObject};

use super::ProfileListItemEvent;

pub(super) struct ProfileNameFieldDelegateIvars {
    id: ProfileId,
    field: Retained<NSTextField>,
    events: UiEventTarget<ProfileListItemEvent>,
}

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "KeyringdInlineProfileNameFieldDelegate"]
    #[ivars = ProfileNameFieldDelegateIvars]
    pub(super) struct ProfileNameFieldDelegate;

    unsafe impl NSObjectProtocol for ProfileNameFieldDelegate {}

    unsafe impl NSControlTextEditingDelegate for ProfileNameFieldDelegate {
        #[unsafe(method(controlTextDidChange:))]
        fn control_text_did_change(&self, _notification: &NSNotification) {
            self.send(ProfileListItemEvent::RenameNameChanged {
                id: self.ivars().id.clone(),
                name: self.ivars().field.stringValue().to_string(),
            });
        }

        #[unsafe(method(control:textView:doCommandBySelector:))]
        unsafe fn control_text_view_do_command(
            &self,
            _control: &NSControl,
            _text_view: &NSTextView,
            command: Sel,
        ) -> bool {
            if command == sel!(insertNewline:) {
                self.send(ProfileListItemEvent::RenameNameChanged {
                    id: self.ivars().id.clone(),
                    name: self.ivars().field.stringValue().to_string(),
                });
                self.send(ProfileListItemEvent::RenameRequested(
                    self.ivars().id.clone(),
                ));
                true
            } else if command == sel!(cancelOperation:) {
                self.send(ProfileListItemEvent::RenameCancelled(
                    self.ivars().id.clone(),
                ));
                true
            } else {
                false
            }
        }
    }

    unsafe impl NSTextFieldDelegate for ProfileNameFieldDelegate {}
);

impl ProfileNameFieldDelegate {
    pub(super) fn new(
        mtm: MainThreadMarker,
        id: ProfileId,
        field: Retained<objc2_app_kit::NSTextField>,
        events: UiEventTarget<ProfileListItemEvent>,
    ) -> Retained<Self> {
        let allocated = mtm
            .alloc()
            .set_ivars(ProfileNameFieldDelegateIvars { id, field, events });
        unsafe { msg_send![super(allocated), init] }
    }

    fn send(&self, event: ProfileListItemEvent) {
        self.ivars().events.send(event);
    }
}
