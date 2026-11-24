use super::QrPromptCommandHandler;
use gtk4::glib::object::{Cast, ObjectExt};
use gtk4::prelude::{AccessibleExtManual, BoxExt, ButtonExt, GtkWindowExt, WidgetExt};

use keyringd_ui_core::qr_prompt_window::HybridQrPresentation;

use keyringd_ui_core::text as ui_text;
use keyringd_ui_core::{
    ComponentModel,
    qr_prompt_window::{QrPromptEvent, QrPromptState},
};
use std::{cell::RefCell, rc::Rc};

/// Presents and dismisses Hybrid QR prompts.
pub(in crate::app) struct QrPromptWindow {
    window: libadwaita::Window,
    state: Rc<RefCell<QrPromptState>>,
    handler: QrPromptCommandHandler,
}

impl QrPromptWindow {
    pub(in crate::app) fn present(
        application: &libadwaita::Application,
        parent: Option<&gtk4::Window>,
        qr_code: &gtk4::Widget,
        presentation: &HybridQrPresentation,
        prompt_id: keyringd_fido_client::InteractionId,
        events: keyringd_ui_core::runtime::UiEventSender<
            keyringd_ui_core::key_operation::KeyOperationEvent,
        >,
    ) -> Self {
        let state = Rc::new(RefCell::new(QrPromptState::default()));
        let content = gtk4::Box::new(gtk4::Orientation::Vertical, 12);
        content.set_margin_top(24);
        content.set_margin_bottom(24);
        content.set_margin_start(24);
        content.set_margin_end(24);
        let heading = gtk4::Label::builder()
            .label(presentation.title())
            .wrap(true)
            .justify(gtk4::Justification::Center)
            .build();
        heading.add_css_class("title-2");
        let body = gtk4::Label::builder()
            .label(presentation.explanation())
            .wrap(true)
            .justify(gtk4::Justification::Center)
            .build();
        content.append(&heading);
        content.append(&body);
        let use_security_key = gtk4::Button::with_label(ui_text::QR_PROMPT_USE_SECURITY_KEY);
        use_security_key.set_halign(gtk4::Align::Start);
        use_security_key.set_visible(presentation.security_key_available);
        let qr_section = gtk4::Box::new(gtk4::Orientation::Vertical, 12);
        qr_section.set_halign(gtk4::Align::Center);
        qr_section.append(qr_code);
        qr_section.append(&use_security_key);
        content.append(&qr_section);
        let cancel = gtk4::Button::with_label(ui_text::CANCEL);
        let actions = gtk4::ActionBar::new();
        actions.pack_end(&cancel);
        let toolbar = libadwaita::ToolbarView::builder()
            .accessible_role(gtk4::AccessibleRole::Group)
            .build();
        toolbar
            .upcast_ref::<gtk4::Widget>()
            .update_property(&[gtk4::accessible::Property::Label(
                ui_text::PROFILE_SETUP_CHOICES_ACCESSIBILITY,
            )]);
        toolbar.add_top_bar(&libadwaita::HeaderBar::builder().show_title(false).build());
        toolbar.set_content(Some(&content));
        toolbar.add_bottom_bar(&actions);
        let dialog = libadwaita::Window::builder()
            .title(presentation.title())
            .default_width(crate::app::ui::PASSKEY_DIALOG_WIDTH)
            .resizable(false)
            .modal(true)
            .accessible_role(gtk4::AccessibleRole::Dialog)
            .content(&toolbar)
            .build();
        dialog.set_application(Some(application));
        if let Some(parent) = parent {
            dialog.set_transient_for(Some(parent));
        }
        let handler = QrPromptCommandHandler::new(dialog.downgrade(), prompt_id, events);
        let usb_state = Rc::clone(&state);
        let usb_handler = handler.clone();
        use_security_key.connect_clicked(move |_| {
            if let Some(command) = usb_state
                .borrow_mut()
                .update(QrPromptEvent::UseUsbRequested)
            {
                usb_handler.handle(command);
            }
        });
        let weak_dialog = dialog.downgrade();
        cancel.connect_clicked(move |_| {
            if let Some(dialog) = weak_dialog.upgrade() {
                dialog.close();
            }
        });
        let escape = gtk4::EventControllerKey::new();
        let weak_dialog = dialog.downgrade();
        escape.connect_key_pressed(move |_, key, _, _| {
            if key == gtk4::gdk::Key::Escape {
                if let Some(dialog) = weak_dialog.upgrade() {
                    dialog.close();
                }
                gtk4::glib::Propagation::Stop
            } else {
                gtk4::glib::Propagation::Proceed
            }
        });
        dialog.add_controller(escape);
        let close_state = Rc::clone(&state);
        let close_handler = handler.clone();
        dialog.connect_close_request(move |_| {
            if let Some(command) = close_state
                .borrow_mut()
                .update(QrPromptEvent::CancelRequested)
            {
                close_handler.handle(command);
            }
            gtk4::glib::Propagation::Proceed
        });
        dialog.present();
        tracing::debug!("Hybrid QR window presented");
        Self {
            window: dialog,
            state,
            handler,
        }
    }

    pub(in crate::app) fn usb_selected(&self) {
        self.state.borrow_mut().update(QrPromptEvent::UsbSelected);
        self.window.set_visible(false);
    }

    pub(in crate::app) fn dismiss(&self) {
        if let Some(command) = self.state.borrow_mut().update(QrPromptEvent::Dismissed) {
            self.handler.handle(command);
        }
    }
}
