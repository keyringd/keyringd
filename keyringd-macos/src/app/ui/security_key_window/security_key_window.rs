use keyringd_ui_core::runtime::UiEventSender;
use keyringd_ui_core::text as ui_text;

use objc2::rc::Retained;
use objc2::{MainThreadMarker, sel};
use objc2_app_kit::{NSAlert, NSFloatingWindowLevel, NSWindow};
use objc2_foundation::NSString;

use super::SecurityKeyActionTarget;
use crate::app::ui::AppShell;
use keyringd_ui_core::key_operation::AuthenticatorPromptKind;
use keyringd_ui_core::key_operation::KeyOperationEvent;
use keyringd_ui_core::key_operation::KeyOperationKind;

struct PromptAlert {
    alert: Retained<NSAlert>,
    parent: Option<Retained<NSWindow>>,
}

pub(crate) struct SecurityKeyWindow {
    mtm: MainThreadMarker,
    shell: Retained<AppShell>,
    prompt: Option<PromptAlert>,
    cancel: Option<Retained<SecurityKeyActionTarget>>,
}

impl SecurityKeyWindow {
    pub(crate) fn new(mtm: MainThreadMarker, shell: Retained<AppShell>) -> Self {
        Self {
            mtm,
            shell,
            prompt: None,
            cancel: None,
        }
    }

    pub(crate) fn show_security_key_prompt(
        &mut self,
        operation: &KeyOperationKind,
        prompt_kind: AuthenticatorPromptKind,
        retry: bool,
        events: UiEventSender<KeyOperationEvent>,
    ) {
        self.dismiss();
        let alert = NSAlert::new(self.mtm);
        let (title, explanation) = keyringd_ui_core::security_key_window::operation_prompt_text(
            operation,
            prompt_kind,
            retry,
        );
        alert.setMessageText(&NSString::from_str(title));
        alert.setInformativeText(&NSString::from_str(&explanation));
        let cancel_target = SecurityKeyActionTarget::new(self.mtm, events);
        let cancel = alert.addButtonWithTitle(&NSString::from_str(ui_text::CANCEL));
        unsafe {
            cancel.setTarget(Some(&cancel_target));
            cancel.setAction(Some(sel!(cancelOperation:)));
        }
        let parent = self.present_alert(&alert);
        self.cancel = Some(cancel_target);
        self.prompt = Some(PromptAlert { alert, parent });
    }

    pub(crate) fn dismiss(&mut self) {
        self.cancel = None;
        if let Some(prompt) = self.prompt.take() {
            let window = prompt.alert.window();
            if let Some(parent) = prompt.parent.as_deref() {
                parent.endSheet(&window);
            }
            window.orderOut(None);
            self.shell.window_hidden();
        }
    }

    fn present_alert(&self, alert: &NSAlert) -> Option<Retained<NSWindow>> {
        match self.shell.anchor() {
            Some(parent) => {
                alert.beginSheetModalForWindow_completionHandler(&parent, None);
                Some(parent)
            }
            None => {
                self.shell.request_attention();
                // Directly showing the alert's window bypasses the sheet API's
                // automatic layout. Finalize text and controls before displaying it.
                alert.layout();
                let window = alert.window();
                window.setLevel(NSFloatingWindowLevel);
                window.makeKeyAndOrderFront(None);
                None
            }
        }
    }
}
