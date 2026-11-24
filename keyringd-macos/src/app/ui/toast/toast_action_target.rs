use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send};
use objc2_foundation::NSObject;

pub(super) struct ToastActionTargetIvars {
    dismiss: Box<dyn Fn()>,
}

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "KeyringdToastActionTarget"]
    #[ivars = ToastActionTargetIvars]
    pub(super) struct ToastActionTarget;

    impl ToastActionTarget {
        #[unsafe(method(dismissToast:))]
        fn dismiss_toast(&self, _sender: Option<&AnyObject>) {
            (self.ivars().dismiss)();
        }
    }
);

impl ToastActionTarget {
    pub(super) fn new(mtm: MainThreadMarker, dismiss: impl Fn() + 'static) -> Retained<Self> {
        let allocated = mtm.alloc().set_ivars(ToastActionTargetIvars {
            dismiss: Box::new(dismiss),
        });
        unsafe { msg_send![super(allocated), init] }
    }
}
