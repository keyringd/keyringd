use super::{ProfileSetupPinView, ProfileSetupQrView, ProfileSetupWaitingView};
use gtk4::prelude::{AccessibleExtManual, ButtonExt, WidgetExt};
use keyringd_ui_core::profile_setup_window::ProfileSetupEvent;
use keyringd_ui_core::profile_setup_window::{
    ProfileSetupInteractionPresentation, ProfileSetupInteractionScreen,
};
use keyringd_ui_core::runtime::UiEventTarget;
#[cfg(test)]
use keyringd_ui_core::text as ui_text;
use std::{cell::Cell, rc::Rc};

pub(in crate::app::ui) struct ProfileSetupInteractionPage {
    operation_id: Rc<Cell<Option<keyringd_fido_client::OperationId>>>,
    root: gtk4::Stack,
    qr: ProfileSetupQrView,
    pin: ProfileSetupPinView,
    waiting: ProfileSetupWaitingView,
}

impl ProfileSetupInteractionPage {
    pub(in crate::app::ui) fn new(events: UiEventTarget<ProfileSetupEvent>) -> Self {
        let qr = ProfileSetupQrView::new();
        let pin = ProfileSetupPinView::new(events.clone());
        let waiting = ProfileSetupWaitingView::new();
        for button in [&qr.cancel, &pin.cancel, &waiting.cancel] {
            let events = events.clone();
            button.connect_clicked(move |_| events.send(ProfileSetupEvent::Dismissed));
        }
        let usb_events = events;
        let operation_id = Rc::new(Cell::new(None));
        let usb_operation = operation_id.clone();
        qr.use_security_key.connect_clicked(move |_| {
            if let Some(id) = usb_operation.get() {
                usb_events.send(ProfileSetupEvent::UseSecurityKeyRequested(id));
            }
        });
        let root = gtk4::Stack::builder()
            .vhomogeneous(true)
            .accessible_role(gtk4::AccessibleRole::Group)
            .build();
        root.update_property(&[gtk4::accessible::Property::Label(
            "Passkey setup interaction",
        )]);
        root.add_child(qr.widget());
        root.add_child(pin.widget());
        root.add_child(waiting.widget());
        Self {
            operation_id,
            root,
            qr,
            pin,
            waiting,
        }
    }

    pub(in crate::app::ui) fn widget(&self) -> &gtk4::Stack {
        &self.root
    }
    pub(in crate::app::ui) fn render(
        &self,
        presentation: &ProfileSetupInteractionPresentation,
        request_id: Option<keyringd_fido_client::InteractionId>,
    ) {
        self.operation_id.set(presentation.operation_id);
        self.waiting.cancel.set_sensitive(!matches!(
            presentation.screen,
            ProfileSetupInteractionScreen::Cancelling
        ));
        if !matches!(presentation.screen, ProfileSetupInteractionScreen::Pin(_)) {
            self.pin.clear_request();
        }
        match &presentation.screen {
            ProfileSetupInteractionScreen::Hidden => {
                self.waiting.stop();
                self.pin.cancel_pending();
            }
            ProfileSetupInteractionScreen::Preparing
            | ProfileSetupInteractionScreen::Cancelling => {
                self.pin.cancel_pending();
                self.waiting.render(presentation);
                self.waiting.header.heading.set_label(&presentation.title());
                self.waiting
                    .header
                    .body
                    .set_label(&presentation.description());
                self.root.set_visible_child(self.waiting.widget());
            }
            ProfileSetupInteractionScreen::Qr(qr) => {
                self.waiting.stop();
                self.qr.render(qr);
                self.qr.header.heading.set_label(&presentation.title());
                self.qr.header.body.set_label(&presentation.description());
                self.root.set_visible_child(self.qr.widget());
            }
            ProfileSetupInteractionScreen::Pin(_) => {
                self.waiting.stop();
                let prompt = presentation
                    .pin_prompt()
                    .expect("PIN screen carries a PIN prompt");
                self.pin.render(&prompt, request_id);
                self.pin
                    .header
                    .heading
                    .set_label(&presentation.stage_title());
                self.pin
                    .header
                    .body
                    .set_label(&presentation.stage_description());
                self.root.set_visible_child(self.pin.widget());
                self.pin.focus();
            }
            ProfileSetupInteractionScreen::Waiting { .. } => {
                self.pin.cancel_pending();
                self.waiting.render(presentation);
                self.waiting
                    .header
                    .heading
                    .set_label(&presentation.stage_title());
                self.waiting
                    .header
                    .body
                    .set_label(&presentation.stage_description());
                self.root.set_visible_child(self.waiting.widget());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gtk4::glib::object::Cast;
    use gtk4::prelude::{EditableExt, WidgetExt};
    use keyringd_ui_core::pin_prompt_window::{PinPromptCommand, SecurityKeyPinPrompt};
    use std::cell::RefCell;
    use std::rc::Rc;

    fn pin_row(widget: &gtk4::Widget) -> Option<libadwaita::PasswordEntryRow> {
        if let Some(row) = widget.downcast_ref::<libadwaita::PasswordEntryRow>() {
            return Some(row.clone());
        }
        let mut child = widget.first_child();
        while let Some(widget) = child {
            if let Some(row) = pin_row(&widget) {
                return Some(row);
            }
            child = widget.next_sibling();
        }
        None
    }
    fn continue_button(widget: &gtk4::Widget) -> Option<gtk4::Button> {
        if let Some(button) = widget.downcast_ref::<gtk4::Button>()
            && button.label().as_deref() == Some(ui_text::CONTINUE)
        {
            return Some(button.clone());
        }
        let mut child = widget.first_child();
        while let Some(widget) = child {
            if let Some(button) = continue_button(&widget) {
                return Some(button);
            }
            child = widget.next_sibling();
        }
        None
    }

    keyringd_testkit::scenario! {
        group "ProfileSetupInteractionPage::render" {
            case "Interaction owns prompt navigation and emits pin completion without a response sender" {
                verify "update wizard navigation and emit the PIN answer through the event route" {
                    gtk4::test_synced(|| {
                        libadwaita::init().unwrap();
                        let events = Rc::new(RefCell::new(Vec::new()));
                        let output = events.clone();
                        let view = ProfileSetupInteractionPage::new(UiEventTarget::new(move |event| {
                            output.borrow_mut().push(event)
                        }));
                        let presentation = ProfileSetupInteractionPresentation {
                            screen: ProfileSetupInteractionScreen::Pin(SecurityKeyPinPrompt {
                                reason: String::new(),
                                attempts_left: None,
                                minimum_length: Some(6),
                                new_pin: false,
                            }),
                            context: None,
                            stage: None,
                            operation_id: None,
                            status: "",
                        };
                        let request = |sequence| {
                            keyringd_fido_client::InteractionId::new(
                                keyringd_fido_client::OperationId::new(1),
                                sequence,
                            )
                        };
                        view.render(&presentation, Some(request(1)));
                        assert_eq!(view.root.visible_child().as_ref(), Some(view.pin.widget()));
                        let pin = pin_row(view.pin.widget()).unwrap();
                        pin.set_text("123456");
                        let submit = continue_button(view.pin.widget()).unwrap();
                        submit.emit_clicked();
                        submit.emit_clicked();
                        assert_eq!(events.borrow().len(), 1);
                        assert!(
                            matches!(&events.borrow()[0], ProfileSetupEvent::PinCompleted { request_id, command: PinPromptCommand::Submit(pin) } if *request_id == request(1) && pin == "123456")
                        );
                        view.render(&presentation, Some(request(2)));
                        assert!(pin_row(view.pin.widget()).unwrap().text().is_empty());
                        view.pin.cancel_pending();
                        assert!(matches!(
                            &events.borrow()[1],
                            ProfileSetupEvent::PinCompleted {
                                request_id,
                                command: PinPromptCommand::Dismiss
                            } if *request_id == request(2)
                        ));
                    });
                }
            }
        }
    }
}
