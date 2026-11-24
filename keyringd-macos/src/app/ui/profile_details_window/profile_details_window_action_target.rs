use keyringd_ui_core::runtime::UiEventSender;
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send};
use objc2_foundation::NSObject;

use super::ProfileDetailsEvent;

pub(super) struct ProfileDetailsWindowActionTargetIvars {
    events: UiEventSender<ProfileDetailsEvent>,
    event: ProfileDetailsEvent,
}

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "KeyringdProfileDetailsWindowActionTarget"]
    #[ivars = ProfileDetailsWindowActionTargetIvars]
    pub(super) struct ProfileDetailsWindowActionTarget;

    impl ProfileDetailsWindowActionTarget {
        #[unsafe(method(performAction:))]
        fn perform_action(&self, _sender: Option<&AnyObject>) {
            self.ivars().events.send(self.ivars().event.clone());
        }
    }
);

impl ProfileDetailsWindowActionTarget {
    pub(super) fn new(
        mtm: MainThreadMarker,
        events: UiEventSender<ProfileDetailsEvent>,
        event: ProfileDetailsEvent,
    ) -> Retained<Self> {
        let allocated = mtm
            .alloc()
            .set_ivars(ProfileDetailsWindowActionTargetIvars { events, event });
        unsafe { msg_send![super(allocated), init] }
    }
}
