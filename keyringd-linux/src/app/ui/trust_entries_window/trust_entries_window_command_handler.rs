use keyringd_ui_core::runtime::UiEventTarget;

use gtk4::gdk::prelude::DisplayExt;
use gtk4::prelude::GtkWindowExt;

use keyringd_ui_core::toast::ToastMessage;
use keyringd_ui_core::trust_entries_window::TrustEntriesCommand;

#[derive(Clone)]
pub(super) struct TrustEntriesWindowCommandHandler {
    on_toast: UiEventTarget<ToastMessage>,
}

impl TrustEntriesWindowCommandHandler {
    pub(super) fn new(on_toast: UiEventTarget<ToastMessage>) -> Self {
        Self { on_toast }
    }

    pub(super) fn handle(&self, command: TrustEntriesCommand, window: &libadwaita::MessageDialog) {
        match command {
            TrustEntriesCommand::Copy {
                value,
                success_message,
            } => {
                if let Some(display) = gtk4::gdk::Display::default() {
                    display.clipboard().set_text(&value);
                    self.on_toast
                        .send(ToastMessage::Success(success_message.to_owned()));
                } else {
                    self.on_toast.send(ToastMessage::Error(
                        keyringd_ui_core::text::COPY_FAILED.to_owned(),
                    ));
                }
            }
            TrustEntriesCommand::Close => window.close(),
        }
    }
}
