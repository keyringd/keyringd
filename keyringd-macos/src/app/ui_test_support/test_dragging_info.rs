//! The dragging information a simulated row drop carries.
//!
//! A real drag session runs AppKit's own event loop on the main thread, which
//! is the thread that answers the driver, so a case cannot start one. The drop
//! is therefore delivered to the product's data source directly, with the
//! pasteboard the product itself filled through `pasteboardWriterForRow:`. Only
//! `draggingPasteboard` is answered, because that is all the product reads; a
//! product change that consults the drag location or the source would find an
//! object that does not answer, rather than a plausible wrong value.

use objc2::rc::Retained;
use objc2::runtime::NSObjectProtocol;
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send};
use objc2_app_kit::NSPasteboard;
use objc2_foundation::NSObject;

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "KeyringdTestDraggingInfo"]
    #[ivars = Retained<NSPasteboard>]
    pub(super) struct TestDraggingInfo;

    unsafe impl NSObjectProtocol for TestDraggingInfo {}

    impl TestDraggingInfo {
        #[unsafe(method_id(draggingPasteboard))]
        fn dragging_pasteboard(&self) -> Retained<NSPasteboard> {
            self.ivars().clone()
        }
    }
);

impl TestDraggingInfo {
    pub(super) fn new(mtm: MainThreadMarker, pasteboard: Retained<NSPasteboard>) -> Retained<Self> {
        let allocated = mtm.alloc().set_ivars(pasteboard);
        unsafe { msg_send![super(allocated), init] }
    }
}
