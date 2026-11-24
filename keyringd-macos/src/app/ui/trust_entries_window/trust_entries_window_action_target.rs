use super::TrustEntriesEvent;
use keyringd_ui_core::runtime::UiEventSender;
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send};
use objc2_foundation::NSObject;
pub(super) struct TrustEntriesWindowActionTargetIvars {
    events: UiEventSender<TrustEntriesEvent>,
    event: TrustEntriesEvent,
}

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "KeyringdTrustEntriesActionTarget"]
    #[ivars = TrustEntriesWindowActionTargetIvars]
    pub(super) struct TrustEntriesWindowActionTarget;
    impl TrustEntriesWindowActionTarget {
        #[unsafe(method(performAction:))]
        fn perform_action(&self, _sender: Option<&AnyObject>) {
            self.ivars().events.send(self.ivars().event.clone());
        }
    }
);
impl TrustEntriesWindowActionTarget {
    pub(super) fn new(
        mtm: MainThreadMarker,
        events: UiEventSender<TrustEntriesEvent>,
        event: TrustEntriesEvent,
    ) -> Retained<Self> {
        let allocated = mtm
            .alloc()
            .set_ivars(TrustEntriesWindowActionTargetIvars { events, event });
        unsafe { msg_send![super(allocated), init] }
    }
}
