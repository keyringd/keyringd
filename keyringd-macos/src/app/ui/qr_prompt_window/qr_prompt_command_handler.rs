use crate::app::ui::AppShell;
use keyringd_fido_client::InteractionId;
use keyringd_ui_core::{
    key_operation::KeyOperationEvent, qr_prompt_window::QrPromptCommand, runtime::UiEventSender,
};
use objc2::rc::Retained;
use objc2_app_kit::NSWindow;

pub(super) struct QrPromptCommandHandler {
    events: UiEventSender<KeyOperationEvent>,
    prompt_id: InteractionId,
    window: Retained<NSWindow>,
    parent: Option<Retained<NSWindow>>,
    shell: Retained<AppShell>,
}
impl QrPromptCommandHandler {
    pub(super) fn new(
        events: UiEventSender<KeyOperationEvent>,
        prompt_id: InteractionId,
        window: Retained<NSWindow>,
        shell: Retained<AppShell>,
        parent: Option<Retained<NSWindow>>,
    ) -> Self {
        Self {
            events,
            prompt_id,
            window,
            shell,
            parent,
        }
    }
    pub(super) fn handle(&self, command: QrPromptCommand) {
        match command {
            QrPromptCommand::Cancel => self.events.send(KeyOperationEvent::HybridQrCancelled {
                prompt_id: self.prompt_id,
            }),
            QrPromptCommand::UseUsb => {
                self.events
                    .send(KeyOperationEvent::HybridQrUseSecurityKeyRequested {
                        prompt_id: self.prompt_id,
                    })
            }
            QrPromptCommand::Close => {
                if let Some(parent) = &self.parent
                    && self.window.sheetParent().is_some()
                {
                    parent.endSheet(&self.window);
                }
                self.window.orderOut(None);
                self.shell.window_hidden();
            }
        }
    }
}
