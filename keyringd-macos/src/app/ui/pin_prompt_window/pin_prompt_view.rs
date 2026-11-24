use super::{PinPromptActionTarget, PinPromptFieldDelegate};
use crate::app::ui::Layout;
use keyringd_ui_core::ComponentModel;
use keyringd_ui_core::pin_prompt_window::{
    PinPromptCommand, PinPromptEvent, PinPromptState, SecurityKeyPinPrompt,
};
use keyringd_ui_core::runtime::UiEventTarget;
use keyringd_ui_core::text as ui_text;
use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2::{MainThreadMarker, Message, sel};
use objc2_app_kit::{
    NSAccessibility, NSButton, NSColor, NSFont, NSLayoutAttribute, NSSecureTextField, NSStackView,
    NSTextField, NSUserInterfaceLayoutOrientation, NSView,
};
use objc2_foundation::{NSArray, NSString};
use std::cell::RefCell;
use std::rc::Rc;

/// Owns PIN input state for standalone windows and embedded wizard prompts.
pub(crate) struct PinPromptView {
    root: Retained<NSStackView>,
    pin: Retained<NSSecureTextField>,
    confirmation: Retained<NSSecureTextField>,
    feedback: Retained<NSTextField>,
    submit: Retained<NSButton>,
    state: RefCell<Option<PinPromptState>>,
    prompt: RefCell<Option<SecurityKeyPinPrompt>>,
    on_command: UiEventTarget<PinPromptCommand>,
    _delegates: Vec<Retained<PinPromptFieldDelegate>>,
    _targets: Vec<Retained<PinPromptActionTarget>>,
}
impl PinPromptView {
    pub(crate) fn new(
        submit: &NSButton,
        cancel: Option<&NSButton>,
        identifier: &str,
        on_command: UiEventTarget<PinPromptCommand>,
        mtm: MainThreadMarker,
    ) -> Rc<Self> {
        let pin = NSSecureTextField::new(mtm);
        pin.setPlaceholderString(Some(&NSString::from_str(ui_text::PIN_PROMPT_PIN_LABEL)));
        pin.setAccessibilityLabel(Some(&NSString::from_str(ui_text::PIN_PROMPT_PIN_LABEL)));
        pin.setAccessibilityIdentifier(Some(&NSString::from_str(&format!("{identifier}.pin"))));
        let confirmation = NSSecureTextField::new(mtm);
        confirmation.setPlaceholderString(Some(&NSString::from_str(
            ui_text::PIN_PROMPT_CONFIRM_PIN_LABEL,
        )));
        confirmation.setAccessibilityLabel(Some(&NSString::from_str(
            ui_text::PIN_PROMPT_CONFIRM_PIN_LABEL,
        )));
        confirmation.setAccessibilityIdentifier(Some(&NSString::from_str(&format!(
            "{identifier}.confirm-pin"
        ))));
        let feedback = NSTextField::wrappingLabelWithString(&NSString::new(), mtm);
        feedback.setFont(Some(&NSFont::systemFontOfSize(13.0)));
        feedback.setTextColor(Some(&NSColor::systemRedColor()));
        feedback.setAccessibilityIdentifier(Some(&NSString::from_str(&format!(
            "{identifier}.pin-validation"
        ))));
        let root = NSStackView::stackViewWithViews(
            &NSArray::from_slice(&[&*pin as &NSView, &*confirmation, &*feedback]),
            mtm,
        );
        root.setOrientation(NSUserInterfaceLayoutOrientation::Vertical);
        root.setAlignment(NSLayoutAttribute::Leading);
        root.setSpacing(12.0);
        for field in [&*pin, &*confirmation] {
            Layout::fill_width(field, &root);
            Layout::height(field, 28.0);
        }
        Layout::fill_width(&feedback, &root);
        let view = Rc::new_cyclic(|weak: &std::rc::Weak<Self>| {
            let weak = weak.clone();
            let events = UiEventTarget::new(move |event| {
                if let Some(view) = weak.upgrade() {
                    view.dispatch(event);
                }
            });
            let mut delegates = Vec::new();
            for (field, is_confirmation) in [(&pin, false), (&confirmation, true)] {
                let delegate = PinPromptFieldDelegate::new(
                    mtm,
                    field.clone(),
                    is_confirmation,
                    events.clone(),
                );
                unsafe {
                    field.setDelegate(Some(ProtocolObject::from_ref(&*delegate)));
                }
                delegates.push(delegate);
            }
            let mut targets = Vec::new();
            for (button, submits) in
                std::iter::once((submit, true)).chain(cancel.map(|button| (button, false)))
            {
                let target = PinPromptActionTarget::new(mtm, events.clone(), submits);
                unsafe {
                    button.setTarget(Some(&target));
                    button.setAction(Some(sel!(performAction:)));
                }
                targets.push(target);
            }
            Self {
                root,
                pin,
                confirmation,
                feedback,
                submit: submit.retain(),
                state: RefCell::new(None),
                prompt: RefCell::new(None),
                on_command,
                _delegates: delegates,
                _targets: targets,
            }
        });
        view.render(None);
        view
    }
    pub(crate) fn view(&self) -> &NSView {
        &self.root
    }
    pub(crate) fn render(&self, prompt: Option<&SecurityKeyPinPrompt>) {
        let changed = self.prompt.borrow().as_ref() != prompt;
        if changed {
            self.state.replace(prompt.map(PinPromptState::for_prompt));
            self.prompt.replace(prompt.cloned());
        }
        self.root.setHidden(prompt.is_none());
        self.render_state();
        if changed
            && prompt.is_some()
            && let Some(window) = self.pin.window()
        {
            window.makeFirstResponder(Some(&self.pin));
        }
    }
    /// A new response channel means a new request, even when its copy is unchanged.
    pub(crate) fn reset(&self) {
        let state = self
            .prompt
            .borrow()
            .as_ref()
            .map(PinPromptState::for_prompt);
        self.state.replace(state);
        self.render_state();
    }
    pub(crate) fn dispatch(&self, event: PinPromptEvent) {
        let command = self
            .state
            .borrow_mut()
            .as_mut()
            .and_then(|state| state.update(event));
        self.render_state();
        if let Some(command) = command {
            self.on_command.send(command);
        }
    }
    fn render_state(&self) {
        let (pin, confirmation, requires_confirmation, can_submit, message) = {
            let state = self.state.borrow();
            (
                state.as_ref().map_or("", PinPromptState::pin).to_owned(),
                state
                    .as_ref()
                    .map_or("", PinPromptState::confirmation)
                    .to_owned(),
                state
                    .as_ref()
                    .is_some_and(PinPromptState::requires_confirmation),
                state.as_ref().is_some_and(PinPromptState::can_submit),
                state.as_ref().and_then(PinPromptState::validation_message),
            )
        };
        if self.pin.stringValue().to_string() != pin {
            self.pin.setStringValue(&NSString::from_str(&pin));
        }
        if self.confirmation.stringValue().to_string() != confirmation {
            self.confirmation
                .setStringValue(&NSString::from_str(&confirmation));
        }
        self.confirmation.setHidden(!requires_confirmation);
        self.submit.setEnabled(can_submit);
        self.feedback.setHidden(message.is_none());
        self.feedback
            .setStringValue(&NSString::from_str(message.unwrap_or("")));
    }
}
