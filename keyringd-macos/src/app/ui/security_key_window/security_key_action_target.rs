use keyringd_ui_core::key_operation::KeyOperationEvent;
use keyringd_ui_core::runtime::UiEventSender;
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send};
use objc2_foundation::NSObject;

pub(super) struct SecurityKeyActionTargetIvars {
    events: UiEventSender<KeyOperationEvent>,
}
define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "KeyringdSecurityKeyActionTarget"]
    #[ivars = SecurityKeyActionTargetIvars]
    pub(super) struct SecurityKeyActionTarget;

    impl SecurityKeyActionTarget {
        #[unsafe(method(cancelOperation:))]
        fn cancel_operation(&self, _sender: Option<&AnyObject>) {
            self.ivars().events.send(KeyOperationEvent::CancelRequested);
        }
    }
);

impl SecurityKeyActionTarget {
    pub(super) fn new(
        mtm: MainThreadMarker,
        events: UiEventSender<KeyOperationEvent>,
    ) -> Retained<Self> {
        let allocated = mtm
            .alloc()
            .set_ivars(SecurityKeyActionTargetIvars { events });
        unsafe { msg_send![super(allocated), init] }
    }
}
