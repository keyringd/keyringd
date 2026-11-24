use gtk4::glib::WeakRef;
use gtk4::prelude::GtkWindowExt;
use keyringd_fido_client::InteractionId;
use keyringd_ui_core::{
    key_operation::KeyOperationEvent, qr_prompt_window::QrPromptCommand, runtime::UiEventSender,
};

#[derive(Clone)]
pub(super) struct QrPromptCommandHandler {
    window: WeakRef<libadwaita::Window>,
    prompt_id: InteractionId,
    events: UiEventSender<KeyOperationEvent>,
}
impl QrPromptCommandHandler {
    pub(super) fn new(
        window: WeakRef<libadwaita::Window>,
        prompt_id: InteractionId,
        events: UiEventSender<KeyOperationEvent>,
    ) -> Self {
        Self {
            window,
            prompt_id,
            events,
        }
    }
    pub(super) fn handle(&self, command: QrPromptCommand) {
        match command {
            QrPromptCommand::UseUsb => {
                self.events
                    .send(KeyOperationEvent::HybridQrUseSecurityKeyRequested {
                        prompt_id: self.prompt_id,
                    })
            }
            QrPromptCommand::Cancel => self.events.send(KeyOperationEvent::HybridQrCancelled {
                prompt_id: self.prompt_id,
            }),
            QrPromptCommand::Close => {
                if let Some(window) = self.window.upgrade() {
                    window.destroy();
                }
            }
        }
    }
}
