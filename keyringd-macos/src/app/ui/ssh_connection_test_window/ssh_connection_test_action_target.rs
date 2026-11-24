use super::SshConnectionTestEvent;
use keyringd_ui_core::runtime::UiEventSender;
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send};
use objc2_foundation::NSObject;
pub(super) struct SshConnectionTestActionTargetIvars {
    events: UiEventSender<SshConnectionTestEvent>,
    event: SshConnectionTestEvent,
}

define_class!(#[unsafe(super(NSObject))] #[thread_kind = MainThreadOnly] #[name="KeyringdConnectionTestActionTarget"] #[ivars=SshConnectionTestActionTargetIvars] pub(super) struct SshConnectionTestActionTarget; impl SshConnectionTestActionTarget { #[unsafe(method(performAction:))] fn perform_action(&self,_sender:Option<&AnyObject>){self.ivars().events.send(self.ivars().event.clone());} });
impl SshConnectionTestActionTarget {
    pub(super) fn new(
        mtm: MainThreadMarker,
        events: UiEventSender<SshConnectionTestEvent>,
        event: SshConnectionTestEvent,
    ) -> Retained<Self> {
        let allocated = mtm
            .alloc()
            .set_ivars(SshConnectionTestActionTargetIvars { events, event });
        unsafe { msg_send![super(allocated), init] }
    }
}
