use super::SshClientConfigurationEvent;
use keyringd_ui_core::runtime::UiEventSender;
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send};
use objc2_foundation::NSObject;
pub(super) struct SshClientConfigurationActionTargetIvars {
    events: UiEventSender<SshClientConfigurationEvent>,
}

define_class!(
 #[unsafe(super(NSObject))]
 #[thread_kind = MainThreadOnly]
 #[name = "KeyringdSshClientConfigurationActionTarget"]
 #[ivars = SshClientConfigurationActionTargetIvars]
 pub(super) struct SshClientConfigurationActionTarget;
 impl SshClientConfigurationActionTarget { #[unsafe(method(performAction:))] fn perform_action(&self, _sender: Option<&AnyObject>) { self.ivars().events.send(SshClientConfigurationEvent::ConfigureRequested); } }
);
impl SshClientConfigurationActionTarget {
    pub(super) fn new(
        mtm: MainThreadMarker,
        events: UiEventSender<SshClientConfigurationEvent>,
    ) -> Retained<Self> {
        let allocated = mtm
            .alloc()
            .set_ivars(SshClientConfigurationActionTargetIvars { events });
        unsafe { msg_send![super(allocated), init] }
    }
}
