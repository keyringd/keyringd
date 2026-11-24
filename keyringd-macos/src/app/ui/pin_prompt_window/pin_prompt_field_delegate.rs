use keyringd_ui_core::pin_prompt_window::PinPromptEvent;
use keyringd_ui_core::runtime::UiEventTarget;
use objc2::rc::Retained;
use objc2::runtime::{NSObjectProtocol, Sel};
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send, sel};
use objc2_app_kit::{
    NSControl, NSControlTextEditingDelegate, NSSecureTextField, NSTextFieldDelegate, NSTextView,
};
use objc2_foundation::{NSNotification, NSObject};

pub(super) struct PinPromptFieldDelegateIvars {
    field: Retained<NSSecureTextField>,
    confirmation: bool,
    events: UiEventTarget<PinPromptEvent>,
}

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "KeyringdPinPromptFieldDelegate"]
    #[ivars = PinPromptFieldDelegateIvars]
    pub(super) struct PinPromptFieldDelegate;

    unsafe impl NSObjectProtocol for PinPromptFieldDelegate {}
    unsafe impl NSControlTextEditingDelegate for PinPromptFieldDelegate {
        #[unsafe(method(controlTextDidChange:))]
        fn control_text_did_change(&self, _notification: &NSNotification) {
            let value = self.ivars().field.stringValue().to_string();
            let event = if self.ivars().confirmation {
                PinPromptEvent::ConfirmationChanged(value)
            } else {
                PinPromptEvent::PinChanged(value)
            };
            self.ivars().events.send(event);
        }
        #[unsafe(method(control:textView:doCommandBySelector:))]
        unsafe fn control_text_view_do_command(
            &self,
            _control: &NSControl,
            _text_view: &NSTextView,
            command: Sel,
        ) -> bool {
            if command == sel!(insertNewline:) {
                self.ivars().events.send(PinPromptEvent::SubmitRequested);
                true
            } else {
                false
            }
        }
    }
    unsafe impl NSTextFieldDelegate for PinPromptFieldDelegate {}
);
impl PinPromptFieldDelegate {
    pub(super) fn new(
        mtm: MainThreadMarker,
        field: Retained<NSSecureTextField>,
        confirmation: bool,
        events: UiEventTarget<PinPromptEvent>,
    ) -> Retained<Self> {
        let allocated = mtm.alloc().set_ivars(PinPromptFieldDelegateIvars {
            field,
            confirmation,
            events,
        });
        unsafe { msg_send![super(allocated), init] }
    }
}
