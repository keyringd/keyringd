use super::PinPromptView;
use crate::app::ui::{AppShell, Layout};
use keyringd_ui_core::key_operation::KeyOperationEvent;
use keyringd_ui_core::pin_prompt_window::{PinPromptCommand, PinPromptEvent, SecurityKeyPinPrompt};
use keyringd_ui_core::runtime::{UiEventSender, UiEventTarget};
use keyringd_ui_core::text as ui_text;
use objc2::rc::Retained;
use objc2::{MainThreadMarker, MainThreadOnly};
use objc2_app_kit::{NSAccessibility, NSAlert, NSView};
use objc2_foundation::{NSPoint, NSRect, NSSize, NSString};
use std::cell::RefCell;
use std::rc::Rc;
use tokio::sync::oneshot;

pub(crate) struct PinPromptWindow {
    alert: Retained<NSAlert>,
    view: Rc<PinPromptView>,
    shell: Retained<AppShell>,
    response: RefCell<Option<oneshot::Sender<Option<String>>>>,
    events: UiEventSender<KeyOperationEvent>,
    request_id: keyringd_fido_client::InteractionId,
}
impl PinPromptWindow {
    pub(crate) fn present(
        mtm: MainThreadMarker,
        shell: Retained<AppShell>,
        prompt: SecurityKeyPinPrompt,
        request_id: keyringd_fido_client::InteractionId,
        response: oneshot::Sender<Option<String>>,
        events: UiEventSender<KeyOperationEvent>,
    ) -> Rc<Self> {
        let alert = NSAlert::new(mtm);
        alert.setMessageText(&NSString::from_str(
            keyringd_ui_core::pin_prompt_window::pin_heading(prompt.new_pin),
        ));
        alert.setInformativeText(&NSString::from_str(
            &keyringd_ui_core::pin_prompt_window::pin_explanation(
                &prompt.reason,
                prompt.minimum_length,
                prompt.attempts_left,
            ),
        ));
        let submit = alert.addButtonWithTitle(&NSString::from_str(ui_text::CONTINUE));
        submit.setAccessibilityIdentifier(Some(&NSString::from_str("key-operation.pin-submit")));
        let cancel = alert.addButtonWithTitle(&NSString::from_str(ui_text::CANCEL));
        let window = Rc::new_cyclic(|weak: &std::rc::Weak<Self>| {
            let weak = weak.clone();
            let on_command = UiEventTarget::new(move |command| {
                if let Some(window) = weak.upgrade() {
                    match command {
                        PinPromptCommand::Submit(value) => window.finish(Some(value)),
                        PinPromptCommand::Cancel | PinPromptCommand::Dismiss => window.finish(None),
                    }
                }
            });
            let view = PinPromptView::new(&submit, Some(&cancel), "key-operation", on_command, mtm);
            let accessory = NSView::initWithFrame(
                NSView::alloc(mtm),
                NSRect::new(
                    NSPoint::new(0.0, 0.0),
                    NSSize::new(336.0, if prompt.new_pin { 124.0 } else { 84.0 }),
                ),
            );
            accessory.addSubview(view.view());
            Layout::pin_top(view.view(), &accessory);
            view.render(Some(&prompt));
            alert.setAccessoryView(Some(&accessory));
            alert.layout();
            Self {
                alert,
                view,
                shell,
                response: RefCell::new(Some(response)),
                events,
                request_id,
            }
        });
        let native = window.alert.window();
        if let Some(parent) = window.shell.anchor() {
            parent.beginSheet_completionHandler(&native, None);
        } else {
            window.shell.request_attention();
            native.center();
            native.makeKeyAndOrderFront(None);
        }
        window
    }
    pub(crate) fn dismiss(&self) {
        self.view.dispatch(PinPromptEvent::Dismissed);
    }
    fn finish(&self, value: Option<String>) {
        let Some(response) = self.response.borrow_mut().take() else {
            return;
        };
        let window = self.alert.window();
        if let Some(parent) = window.sheetParent() {
            parent.endSheet(&window);
        }
        window.orderOut(None);
        self.shell.window_hidden();
        self.events.send(KeyOperationEvent::PinAnswered {
            request_id: self.request_id,
            value,
            response,
        });
    }
}
impl Drop for PinPromptWindow {
    fn drop(&mut self) {
        self.finish(None);
    }
}
