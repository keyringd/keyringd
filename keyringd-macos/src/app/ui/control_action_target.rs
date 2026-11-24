use keyringd_ui_core::runtime::UiEventTarget;
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send};
use objc2_foundation::NSObject;

pub(crate) struct ControlActionTargetIvars {
    action: UiEventTarget<()>,
}

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "KeyringdControlActionTarget"]
    #[ivars = ControlActionTargetIvars]
    pub(crate) struct ControlActionTarget;

    impl ControlActionTarget {
        #[unsafe(method(performAction:))]
        fn perform_action(&self, _sender: Option<&AnyObject>) {
            self.ivars().action.send(());
        }
    }
);

impl ControlActionTarget {
    pub(crate) fn new<Event: Clone + 'static>(
        mtm: MainThreadMarker,
        events: UiEventTarget<Event>,
        event: Event,
    ) -> Retained<Self> {
        let allocated = mtm.alloc().set_ivars(ControlActionTargetIvars {
            action: UiEventTarget::new(move |()| events.send(event.clone())),
        });
        unsafe { msg_send![super(allocated), init] }
    }
}
