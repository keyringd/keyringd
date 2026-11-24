use super::PinPromptView;
use gtk4::glib::object::ObjectExt;
use gtk4::prelude::GtkWindowExt;
use keyringd_ui_core::pin_prompt_window::{PinPromptCommand, PinPromptEvent, SecurityKeyPinPrompt};
use keyringd_ui_core::runtime::{UiEventSender, UiEventTarget};
use keyringd_ui_core::text as ui_text;
use libadwaita::prelude::MessageDialogExt;
use std::cell::RefCell;
use std::rc::Rc;

pub(in crate::app) struct PinPromptWindow {
    dialog: libadwaita::MessageDialog,
}

impl PinPromptWindow {
    pub(in crate::app) fn present(
        application: &libadwaita::Application,
        parent: Option<&gtk4::Window>,
        prompt: &SecurityKeyPinPrompt,
        request_id: keyringd_fido_client::InteractionId,
        response: tokio::sync::oneshot::Sender<Option<String>>,
        events: UiEventSender<keyringd_ui_core::key_operation::KeyOperationEvent>,
    ) -> Self {
        let dialog = crate::app::ui::message_dialog_builder()
            .heading(keyringd_ui_core::pin_prompt_window::pin_heading(
                prompt.new_pin,
            ))
            .close_response("cancel")
            .default_response("continue")
            .build();
        dialog.set_application(Some(application));
        if let Some(parent) = parent {
            dialog.set_transient_for(Some(parent));
        }
        dialog.add_response("cancel", ui_text::CANCEL);
        dialog.add_response("continue", ui_text::CONTINUE);
        dialog.set_response_appearance("continue", libadwaita::ResponseAppearance::Suggested);
        dialog.set_response_enabled("continue", false);
        let response = RefCell::new(Some(response));
        let weak_dialog = dialog.downgrade();
        let view = PinPromptView::new(
            UiEventTarget::new(move |command| {
                let response = response.borrow_mut().take();
                if let Some(response) = response {
                    let value = match command {
                        PinPromptCommand::Submit(pin) => Some(pin),
                        PinPromptCommand::Cancel | PinPromptCommand::Dismiss => None,
                    };
                    events.send(
                        keyringd_ui_core::key_operation::KeyOperationEvent::PinAnswered {
                            request_id,
                            value,
                            response,
                        },
                    );
                    if let Some(dialog) = weak_dialog.upgrade() {
                        dialog.close();
                    }
                }
            }),
            {
                let dialog = dialog.downgrade();
                UiEventTarget::new(move |enabled| {
                    if let Some(dialog) = dialog.upgrade() {
                        dialog.set_response_enabled("continue", enabled);
                    }
                })
            },
        );
        view.render(prompt);
        dialog.set_extra_child(Some(&view.root));
        let response_view = Rc::clone(&view);
        dialog.connect_response(None, move |_, selected| {
            response_view.dispatch(if selected == "continue" {
                PinPromptEvent::SubmitRequested
            } else {
                PinPromptEvent::CancelRequested
            });
        });
        dialog.present();
        Self { dialog }
    }
    pub(in crate::app) fn close(&self) {
        self.dialog.close();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gtk4::prelude::{ApplicationExt, Cast, EditableExt, WidgetExt};
    use keyringd_ui_core::runtime::UiEventSender;

    fn input_rows(widget: &gtk4::Widget, rows: &mut Vec<libadwaita::PasswordEntryRow>) {
        if let Some(row) = widget.downcast_ref::<libadwaita::PasswordEntryRow>() {
            rows.push(row.clone());
            return;
        }
        let mut child = widget.first_child();
        while let Some(widget) = child {
            input_rows(&widget, rows);
            child = widget.next_sibling();
        }
    }

    keyringd_testkit::scenario! {
        group "PinPromptWindow::present" {
            case "Dialog renders confirmation validity and emits one answer event" {
                verify "present the confirmation state and deliver the accepted answer once" {
                    gtk4::test_synced(|| {
                        libadwaita::init().unwrap();
                        let app = libadwaita::Application::builder().build();
                        app.register(None::<&gtk4::gio::Cancellable>).unwrap();
                        let (response, _answer) = tokio::sync::oneshot::channel();
                        let (sender, mut events) = tokio::sync::mpsc::unbounded_channel();
                        let dialog = PinPromptWindow::present(
                            &app,
                            None,
                            &SecurityKeyPinPrompt {
                                reason: String::new(),
                                attempts_left: None,
                                minimum_length: Some(6),
                                new_pin: true,
                            },
                            keyringd_fido_client::InteractionId::new(
                                keyringd_fido_client::OperationId::new(1),
                                2,
                            ),
                            response,
                            UiEventSender::from_tokio(sender),
                        );
                        let dialog = dialog.dialog;
                        let mut rows = Vec::new();
                        input_rows(&dialog.extra_child().unwrap(), &mut rows);
                        assert_eq!(rows.len(), 2);
                        rows[0].set_text("123456");
                        rows[1].set_text("654321");
                        assert!(!dialog.is_response_enabled("continue"));
                        rows[1].set_text("123456");
                        assert!(dialog.is_response_enabled("continue"));
                        dialog.response("continue");
                        assert!(
                            matches!(events.try_recv().unwrap(), keyringd_ui_core::key_operation::KeyOperationEvent::PinAnswered { value: Some(pin), .. } if pin == "123456")
                        );
                        dialog.response("cancel");
                        assert!(events.try_recv().is_err());
                        dialog.close();
                    });
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "PinPromptWindow::present" {
            case "Closing dialog uses the cancel response once" {
                verify "deliver the cancellation response exactly once when the dialog closes" {
                    gtk4::test_synced(|| {
                        libadwaita::init().unwrap();
                        let app = libadwaita::Application::builder()
                            .application_id("com.keyringd.PinPromptCancelTest")
                            .build();
                        app.register(None::<&gtk4::gio::Cancellable>).unwrap();
                        let (response, _answer) = tokio::sync::oneshot::channel();
                        let (sender, mut events) = tokio::sync::mpsc::unbounded_channel();
                        let prompt = PinPromptWindow::present(
                            &app,
                            None,
                            &SecurityKeyPinPrompt {
                                reason: String::new(),
                                attempts_left: None,
                                minimum_length: None,
                                new_pin: false,
                            },
                            keyringd_fido_client::InteractionId::new(
                                keyringd_fido_client::OperationId::new(2),
                                2,
                            ),
                            response,
                            UiEventSender::from_tokio(sender),
                        );
                        prompt.close();
                        assert!(matches!(
                            events.try_recv().unwrap(),
                            keyringd_ui_core::key_operation::KeyOperationEvent::PinAnswered { value: None, .. }
                        ));
                        prompt.close();
                        assert!(events.try_recv().is_err());
                    });
                }
            }
        }
    }
}
