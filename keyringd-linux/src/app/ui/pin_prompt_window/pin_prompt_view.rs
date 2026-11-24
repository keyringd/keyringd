use gtk4::prelude::{BoxExt, EditableExt, WidgetExt};
use keyringd_ui_core::pin_prompt_window::{
    PinPromptCommand, PinPromptEvent, PinPromptState, SecurityKeyPinPrompt,
};
use keyringd_ui_core::runtime::{ComponentRuntime, UiEventTarget};
use keyringd_ui_core::text as ui_text;
use libadwaita::prelude::{EntryRowExt, PreferencesGroupExt};
use std::cell::RefCell;
use std::rc::Rc;

pub(in crate::app::ui) struct PinPromptView {
    pub(in crate::app::ui) root: gtk4::Box,
    explanation: gtk4::Label,
    fields: gtk4::Box,
    pin: RefCell<Option<libadwaita::PasswordEntryRow>>,
    group: RefCell<Option<libadwaita::PreferencesGroup>>,
    prompt: RefCell<Option<SecurityKeyPinPrompt>>,
    runtime: ComponentRuntime<PinPromptState>,
    commands: UiEventTarget<PinPromptCommand>,
    validity: UiEventTarget<bool>,
}

impl PinPromptView {
    pub(in crate::app::ui) fn new(
        commands: UiEventTarget<PinPromptCommand>,
        validity: UiEventTarget<bool>,
    ) -> Rc<Self> {
        let root = gtk4::Box::new(gtk4::Orientation::Vertical, 12);
        let explanation = gtk4::Label::builder().xalign(0.0).wrap(true).build();
        let fields = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        root.append(&explanation);
        root.append(&fields);
        Rc::new(Self {
            root,
            explanation,
            fields,
            pin: RefCell::new(None),
            group: RefCell::new(None),
            prompt: RefCell::new(None),
            runtime: ComponentRuntime::new(PinPromptState::default()),
            commands,
            validity,
        })
    }

    pub(in crate::app::ui) fn dispatch(&self, event: PinPromptEvent) {
        if self.prompt.borrow().is_none() {
            return;
        }
        let command = self.runtime.dispatch_and_render(event, |state| {
            if let Some(group) = self.group.borrow().as_ref() {
                group.set_description(state.validation_message());
            }
        });
        let can_submit = self.runtime.state().can_submit();
        self.validity.send(can_submit);
        if let Some(command) = command {
            self.commands.send(command);
        }
    }

    pub(in crate::app::ui) fn focus(&self) {
        if let Some(pin) = self.pin.borrow().as_ref() {
            pin.grab_focus();
        }
    }

    pub(in crate::app::ui) fn render(self: &Rc<Self>, prompt: &SecurityKeyPinPrompt) {
        if self.prompt.borrow().as_ref() != Some(prompt) {
            self.reset_prompt(prompt);
        }
    }

    pub(in crate::app::ui) fn restart(self: &Rc<Self>) {
        let prompt = self.prompt.borrow().clone();
        if let Some(prompt) = prompt {
            self.reset_prompt(&prompt);
        }
    }

    fn reset_prompt(self: &Rc<Self>, prompt: &SecurityKeyPinPrompt) {
        self.prompt.replace(Some(prompt.clone()));
        self.runtime
            .reset(PinPromptState::for_prompt(prompt), |_| {});
        while let Some(child) = self.fields.first_child() {
            self.fields.remove(&child);
        }
        self.explanation
            .set_label(&keyringd_ui_core::pin_prompt_window::pin_explanation(
                &prompt.reason,
                prompt.minimum_length,
                prompt.attempts_left,
            ));
        let pin = libadwaita::PasswordEntryRow::builder()
            .title(ui_text::PIN_PROMPT_PIN_LABEL)
            .activates_default(true)
            .build();
        pin.connect_map(|pin| {
            pin.grab_focus();
        });
        let group = libadwaita::PreferencesGroup::new();
        group.add(&pin);
        let weak = Rc::downgrade(self);
        pin.connect_changed(move |pin| {
            if let Some(view) = weak.upgrade() {
                view.dispatch(PinPromptEvent::PinChanged(pin.text().to_string()));
            }
        });
        self.connect_activation(&pin);
        if prompt.new_pin {
            let confirmation = libadwaita::PasswordEntryRow::builder()
                .title(ui_text::PIN_PROMPT_CONFIRM_PIN_LABEL)
                .activates_default(true)
                .build();
            let weak = Rc::downgrade(self);
            confirmation.connect_changed(move |row| {
                if let Some(view) = weak.upgrade() {
                    view.dispatch(PinPromptEvent::ConfirmationChanged(row.text().to_string()));
                }
            });
            self.connect_activation(&confirmation);
            group.add(&confirmation);
        }
        self.fields.append(&group);
        self.pin.replace(Some(pin));
        self.group.replace(Some(group));
        self.validity.send(false);
    }

    fn connect_activation(self: &Rc<Self>, row: &libadwaita::PasswordEntryRow) {
        let weak = Rc::downgrade(self);
        row.connect_entry_activated(move |_| {
            if let Some(view) = weak.upgrade() {
                view.dispatch(PinPromptEvent::SubmitRequested);
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gtk4::glib::object::Cast;
    use libadwaita::prelude::PreferencesRowExt;
    use std::cell::Cell;

    fn confirmation_row(widget: &gtk4::Widget) -> Option<libadwaita::PasswordEntryRow> {
        if let Some(row) = widget.downcast_ref::<libadwaita::PasswordEntryRow>()
            && row.title() == ui_text::PIN_PROMPT_CONFIRM_PIN_LABEL
        {
            return Some(row.clone());
        }
        let mut child = widget.first_child();
        while let Some(widget) = child {
            if let Some(row) = confirmation_row(&widget) {
                return Some(row);
            }
            child = widget.next_sibling();
        }
        None
    }

    keyringd_testkit::scenario! {
        group "PinPromptView::render" {
            case "Shared input view resets identical requests and completes once" {
                verify "clear old input for an identical request and send exactly one answer" {
                    gtk4::test_synced(|| {
                        libadwaita::init().unwrap();
                        let commands = Rc::new(RefCell::new(Vec::new()));
                        let output = commands.clone();
                        let enabled = Rc::new(Cell::new(false));
                        let valid = enabled.clone();
                        let view = PinPromptView::new(
                            UiEventTarget::new(move |command| output.borrow_mut().push(command)),
                            UiEventTarget::new(move |value| valid.set(value)),
                        );
                        let prompt = SecurityKeyPinPrompt {
                            reason: String::new(),
                            attempts_left: None,
                            minimum_length: Some(6),
                            new_pin: true,
                        };
                        view.render(&prompt);
                        let pin = view.pin.borrow().as_ref().unwrap().clone();
                        let confirmation = confirmation_row(view.root.upcast_ref()).unwrap();
                        pin.set_text("123456");
                        confirmation.set_text("654321");
                        assert!(!enabled.get());
                        view.dispatch(PinPromptEvent::SubmitRequested);
                        assert!(commands.borrow().is_empty());
                        confirmation.set_text("123456");
                        assert!(enabled.get());
                        view.dispatch(PinPromptEvent::SubmitRequested);
                        view.dispatch(PinPromptEvent::CancelRequested);
                        assert_eq!(
                            *commands.borrow(),
                            vec![PinPromptCommand::Submit("123456".into())]
                        );
                        view.restart();
                        assert!(view.runtime.state().pin().is_empty());
                        assert!(!enabled.get());
                        view.dispatch(PinPromptEvent::Dismissed);
                        view.dispatch(PinPromptEvent::Dismissed);
                        assert_eq!(
                            *commands.borrow(),
                            vec![
                                PinPromptCommand::Submit("123456".into()),
                                PinPromptCommand::Dismiss
                            ]
                        );
                    });
                }
            }
        }
    }
}
