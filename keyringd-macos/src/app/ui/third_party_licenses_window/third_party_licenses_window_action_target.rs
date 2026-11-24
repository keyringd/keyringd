use super::ThirdPartyLicensesWindow;
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send};
use objc2_foundation::NSObject;
use std::rc::Weak;

pub(super) struct ThirdPartyLicensesWindowActionTargetIvars {
    owner: Weak<ThirdPartyLicensesWindow>,
}

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "KeyringdThirdPartyLicensesWindowActionTarget"]
    #[ivars = ThirdPartyLicensesWindowActionTargetIvars]
    pub(super) struct ThirdPartyLicensesWindowActionTarget;
    impl ThirdPartyLicensesWindowActionTarget {
        #[unsafe(method(closeDocument:))]
        fn close_document(&self, _sender: Option<&AnyObject>) {
            if let Some(owner) = self.ivars().owner.upgrade() { owner.close(); }
        }
    }
);
impl ThirdPartyLicensesWindowActionTarget {
    pub(super) fn new(
        mtm: MainThreadMarker,
        owner: Weak<ThirdPartyLicensesWindow>,
    ) -> Retained<Self> {
        let allocated = mtm
            .alloc()
            .set_ivars(ThirdPartyLicensesWindowActionTargetIvars { owner });
        unsafe { msg_send![super(allocated), init] }
    }
}
