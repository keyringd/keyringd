use keyringd_ui_core::runtime::UiEventTarget;
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send};
use objc2_foundation::NSObject;

use super::ProfileSetupEvent;

pub(crate) struct ProfileSetupActionTargetIvars {
    events: UiEventTarget<ProfileSetupEvent>,
    event: std::cell::RefCell<Option<ProfileSetupEvent>>,
}

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "KeyringdProfileSetupActionTarget"]
    #[ivars = ProfileSetupActionTargetIvars]
    pub(crate) struct ProfileSetupActionTarget;

    impl ProfileSetupActionTarget {
        #[unsafe(method(performAction:))]
        fn perform_action(&self, _sender: Option<&AnyObject>) {
            let event = self.ivars().event.borrow().clone();
            if let Some(event) = event {
                self.ivars().events.send(event);
            }
        }
    }
);

impl ProfileSetupActionTarget {
    pub(crate) fn new(
        mtm: MainThreadMarker,
        events: UiEventTarget<ProfileSetupEvent>,
        event: impl Into<Option<ProfileSetupEvent>>,
    ) -> Retained<Self> {
        let allocated = mtm.alloc().set_ivars(ProfileSetupActionTargetIvars {
            events,
            event: std::cell::RefCell::new(event.into()),
        });
        unsafe { msg_send![super(allocated), init] }
    }

    pub(crate) fn set_event(&self, event: Option<ProfileSetupEvent>) {
        self.ivars().event.replace(event);
    }
}
