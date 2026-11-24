use keyringd_ui_core::runtime::UiEventSender;
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send};
use objc2_foundation::NSObject;

use keyringd_ui_core::app_menu::{AppMenuEvent, AppMenuLifecycleEvent};

pub(super) struct MenuBarActionTargetIvars {
    events: UiEventSender<AppMenuLifecycleEvent>,
    event: AppMenuEvent,
}

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "KeyringdMenuBarActionTarget"]
    #[ivars = MenuBarActionTargetIvars]
    pub(super) struct MenuBarActionTarget;

    impl MenuBarActionTarget {
        #[unsafe(method(performAction:))]
        fn perform_action(&self, _sender: Option<&AnyObject>) {
            self.ivars().events.send(AppMenuLifecycleEvent::ActionRequested(self.ivars().event.clone()));
        }
    }
);

impl MenuBarActionTarget {
    pub(super) fn new(
        mtm: MainThreadMarker,
        events: UiEventSender<AppMenuLifecycleEvent>,
        event: AppMenuEvent,
    ) -> Retained<Self> {
        let allocated = mtm
            .alloc()
            .set_ivars(MenuBarActionTargetIvars { events, event });
        unsafe { msg_send![super(allocated), init] }
    }
}
