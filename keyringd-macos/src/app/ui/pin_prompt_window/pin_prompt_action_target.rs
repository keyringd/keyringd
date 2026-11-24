use keyringd_ui_core::pin_prompt_window::PinPromptEvent;
use keyringd_ui_core::runtime::UiEventTarget;
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send};
use objc2_foundation::NSObject;

pub(super) struct PinPromptActionTargetIvars {
    submit: bool,
    events: UiEventTarget<PinPromptEvent>,
}

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "KeyringdPinPromptActionTarget"]
    #[ivars = PinPromptActionTargetIvars]
    pub(super) struct PinPromptActionTarget;
    impl PinPromptActionTarget {
        #[unsafe(method(performAction:))]
        fn perform_action(&self, _sender: Option<&AnyObject>) {
            let event = if self.ivars().submit { PinPromptEvent::SubmitRequested } else { PinPromptEvent::CancelRequested };
            self.ivars().events.send(event);
        }
    }
);
impl PinPromptActionTarget {
    pub(super) fn new(
        mtm: MainThreadMarker,
        events: UiEventTarget<PinPromptEvent>,
        submit: bool,
    ) -> Retained<Self> {
        let allocated = mtm
            .alloc()
            .set_ivars(PinPromptActionTargetIvars { submit, events });
        unsafe { msg_send![super(allocated), init] }
    }
}
