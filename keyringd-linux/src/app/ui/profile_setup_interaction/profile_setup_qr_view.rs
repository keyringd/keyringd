use keyringd_ui_core::text as ui_text;
use std::cell::RefCell;

use gtk4::glib::object::Cast;
use gtk4::prelude::{AccessibleExtManual, BoxExt, WidgetExt};

use crate::app::ui::HYBRID_QR_SIZE;
use crate::app::ui::page_header::PageHeaderView;
use crate::app::ui::profile_setup_interaction::interaction_body;
use crate::app::ui::qr_prompt_window::HybridQrView;
use keyringd_ui_core::profile_setup_window::HybridPrompt;
use keyringd_ui_core::qr_prompt_window::HybridQrPresentation;

pub(super) struct ProfileSetupQrView {
    widget: gtk4::Widget,
    pub(super) cancel: gtk4::Button,
    pub(super) use_security_key: gtk4::Button,
    pub(super) header: PageHeaderView,
    instruction: gtk4::Label,
    content: gtk4::Box,
    rendered: RefCell<Option<HybridPrompt>>,
}

impl ProfileSetupQrView {
    pub(super) fn new() -> Self {
        let body = interaction_body();
        let instruction = gtk4::Label::builder()
            .xalign(0.5)
            .justify(gtk4::Justification::Center)
            .wrap(true)
            .build();
        instruction.set_halign(gtk4::Align::Center);
        instruction.set_size_request(320, -1);
        let content = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        content.set_halign(gtk4::Align::Center);
        content.set_size_request(HYBRID_QR_SIZE, HYBRID_QR_SIZE);
        let use_security_key = gtk4::Button::with_label(ui_text::QR_PROMPT_USE_SECURITY_KEY);
        use_security_key.set_halign(gtk4::Align::Start);
        let qr_section = gtk4::Box::new(gtk4::Orientation::Vertical, 12);
        qr_section.set_halign(gtk4::Align::Center);
        qr_section.append(&content);
        qr_section.append(&use_security_key);
        let cancel = gtk4::Button::with_label(ui_text::CANCEL);
        let actions = gtk4::ActionBar::new();
        actions.pack_end(&cancel);
        body.append(&instruction);
        body.append(&qr_section);
        let header = PageHeaderView::new();
        let wrapper = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        wrapper.append(&header.root);
        wrapper.append(&body);
        let page = libadwaita::ToolbarView::builder()
            .accessible_role(gtk4::AccessibleRole::Group)
            .build();
        page.upcast_ref::<gtk4::Widget>()
            .update_property(&[gtk4::accessible::Property::Label(
                ui_text::PROFILE_SETUP_CHOICES_ACCESSIBILITY,
            )]);
        page.set_content(Some(&wrapper));
        page.add_bottom_bar(&actions);
        Self {
            widget: page.upcast(),
            cancel,
            use_security_key,
            header,
            instruction,
            content,
            rendered: RefCell::new(None),
        }
    }

    pub(super) fn widget(&self) -> &gtk4::Widget {
        &self.widget
    }

    pub(super) fn render(&self, prompt: &HybridPrompt) {
        if self.rendered.borrow().as_ref() == Some(prompt) {
            return;
        }
        self.rendered.replace(Some(prompt.clone()));
        let presentation = HybridQrPresentation::new(
            &prompt.value,
            &prompt.context,
            prompt.security_key_available,
            prompt.unavailable,
        );
        self.instruction.set_label(presentation.instruction());
        self.use_security_key
            .set_visible(presentation.security_key_available);
        self.use_security_key.set_sensitive(true);
        while let Some(child) = self.content.first_child() {
            self.content.remove(&child);
        }
        let qr = HybridQrView::new(&presentation);
        qr.set_qr_accessible_label("Passkey QR code");
        self.content.append(&qr.widget());
    }
}
