//! Clipboard boundary shared by product copy actions and the UI test driver.
use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2_app_kit::NSPasteboard;

#[cfg(feature = "test-support")]
thread_local! {
    static PRIVATE: std::cell::RefCell<Option<Retained<NSPasteboard>>> = const { std::cell::RefCell::new(None) };
}

/// Called only by the fixture entry point. A normal product binary keeps using
/// the user's pasteboard even when built with test-support.
#[cfg(feature = "test-support")]
pub(crate) fn use_private_pasteboard(_mtm: MainThreadMarker) {
    PRIVATE.with(|board| {
        assert!(
            board.borrow().is_none(),
            "fixture pasteboard already installed"
        );
        *board.borrow_mut() = Some(NSPasteboard::pasteboardWithUniqueName());
    });
}

pub(crate) fn pasteboard(_mtm: MainThreadMarker) -> Retained<NSPasteboard> {
    #[cfg(feature = "test-support")]
    if let Some(board) = PRIVATE.with(|board| board.borrow().clone()) {
        return board;
    }
    NSPasteboard::generalPasteboard()
}
