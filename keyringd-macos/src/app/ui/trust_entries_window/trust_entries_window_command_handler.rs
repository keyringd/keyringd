use keyringd_ui_core::trust_entries_window::TrustEntriesCommand;
use objc2::MainThreadMarker;
use objc2_app_kit::{NSPasteboardTypeString, NSWindow};
use objc2_foundation::NSString;

use crate::app::ui::toast::ToastOverlay;
use keyringd_ui_core::toast::ToastMessage;

pub(super) struct TrustEntriesWindowCommandHandler;

impl TrustEntriesWindowCommandHandler {
    pub(super) fn handle(
        command: TrustEntriesCommand,
        window: &NSWindow,
        toast: &ToastOverlay,
        mtm: MainThreadMarker,
    ) {
        match command {
            TrustEntriesCommand::Copy {
                value,
                success_message,
            } => Self::copy(&value, success_message, toast, mtm),
            TrustEntriesCommand::Close => {
                if let Some(parent) = window.sheetParent() {
                    parent.endSheet(window);
                }
            }
        }
    }
    fn copy(value: &str, success_message: &str, toast: &ToastOverlay, mtm: MainThreadMarker) {
        let board = crate::system::clipboard::pasteboard(mtm);
        board.clearContents();
        let copied =
            unsafe { board.setString_forType(&NSString::from_str(value), NSPasteboardTypeString) };
        let message = if copied {
            ToastMessage::Success(success_message.to_owned())
        } else {
            ToastMessage::Error(keyringd_ui_core::text::COPY_FAILED.to_owned())
        };
        toast.show(message, mtm);
    }
}
