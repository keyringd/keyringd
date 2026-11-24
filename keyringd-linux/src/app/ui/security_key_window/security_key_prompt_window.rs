use gtk4::glib::object::ObjectExt;
use gtk4::prelude::{GtkWindowExt, WidgetExt};
use keyringd_ui_core::key_operation::AuthenticatorPromptKind;
use keyringd_ui_core::key_operation::KeyOperationEvent;
use keyringd_ui_core::key_operation::KeyOperationKind;
use keyringd_ui_core::runtime::UiEventSender;
use keyringd_ui_core::security_key_window;
use keyringd_ui_core::text as ui_text;
use libadwaita::prelude::MessageDialogExt;

pub(in crate::app) struct SecurityKeyPromptWindow {
    dialog: libadwaita::MessageDialog,
    response_handler: gtk4::glib::SignalHandlerId,
}

impl SecurityKeyPromptWindow {
    pub(in crate::app) fn new(
        application: &libadwaita::Application,
        parent: Option<&gtk4::Window>,
        operation: &KeyOperationKind,
        prompt_kind: AuthenticatorPromptKind,
        events: UiEventSender<KeyOperationEvent>,
    ) -> Self {
        let spinner = gtk4::Spinner::new();
        spinner.set_halign(gtk4::Align::Center);
        spinner.start();
        let (title, explanation) =
            security_key_window::operation_prompt_text(operation, prompt_kind, false);
        let dialog = crate::app::ui::message_dialog_builder()
            .heading(title)
            .body(explanation)
            .extra_child(&spinner)
            .close_response("cancel")
            .build();
        dialog.set_application(Some(application));
        if let Some(parent) = parent {
            dialog.set_transient_for(Some(parent));
        }
        dialog.add_response("cancel", ui_text::CANCEL);
        let response_handler = dialog.connect_response(None, move |_, _| {
            events.send(KeyOperationEvent::CancelRequested);
        });
        dialog.present();
        Self {
            dialog,
            response_handler,
        }
    }

    pub(in crate::app) fn close(self) {
        // Closing a prompt for a UI transition is not a user cancellation.
        self.dialog.disconnect(self.response_handler);
        self.dialog.close();
    }
}
