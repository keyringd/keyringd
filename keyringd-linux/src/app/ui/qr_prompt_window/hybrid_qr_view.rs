use gtk4::glib::object::Cast;
use gtk4::prelude::{AccessibleExtManual, BoxExt, WidgetExt};
use keyringd_fido_client::HybridTransportUnavailableReason;
use keyringd_ui_core::qr_prompt_window::HybridQrPresentation;
use keyringd_ui_core::text as ui_text;

use crate::app::ui::{HYBRID_QR_SIZE, PASSKEY_DIALOG_WIDTH};

use crate::app::ui::qr_prompt_window::QrCodeView;

// Both prompt containers leave 24 logical pixels on each side.
const CONTENT_WIDTH: i32 = PASSKEY_DIALOG_WIDTH - 48;

#[derive(Clone)]
pub(crate) struct HybridQrView {
    root: gtk4::Box,
    qr_code: gtk4::Widget,
    unavailable_card: gtk4::Box,
    unavailable_message: gtk4::Label,
    security_key_help: gtk4::Box,
}

impl HybridQrView {
    pub(crate) fn new(presentation: &HybridQrPresentation) -> Self {
        let root = gtk4::Box::new(gtk4::Orientation::Vertical, 10);
        root.set_halign(gtk4::Align::Center);

        let overlay = gtk4::Overlay::new();
        overlay.set_size_request(CONTENT_WIDTH, HYBRID_QR_SIZE);

        let qr_code = QrCodeView::render(&presentation.value);
        qr_code.set_halign(gtk4::Align::Center);
        qr_code.set_valign(gtk4::Align::Center);
        overlay.set_child(Some(&qr_code));

        let unavailable_message = gtk4::Label::builder()
            .xalign(0.5)
            .justify(gtk4::Justification::Center)
            .wrap(true)
            .build();
        let unavailable_title = gtk4::Label::builder()
            .label(ui_text::QR_PROMPT_UNAVAILABLE_TITLE)
            .xalign(0.5)
            .css_classes(["heading"])
            .build();
        unavailable_title.set_margin_top(14);
        unavailable_title.set_margin_start(14);
        unavailable_title.set_margin_end(14);
        unavailable_message.set_margin_bottom(14);
        unavailable_message.set_margin_start(14);
        unavailable_message.set_margin_end(14);
        let unavailable_card = gtk4::Box::new(gtk4::Orientation::Vertical, 8);
        unavailable_card.add_css_class("card");
        unavailable_card.set_margin_top(12);
        unavailable_card.set_margin_bottom(12);
        unavailable_card.set_margin_start(12);
        unavailable_card.set_margin_end(12);
        unavailable_card.append(&unavailable_title);
        unavailable_card.append(&unavailable_message);
        unavailable_card.set_halign(gtk4::Align::Fill);
        unavailable_card.set_valign(gtk4::Align::Center);
        unavailable_card.set_visible(false);
        overlay.add_overlay(&unavailable_card);

        let security_key_heading = gtk4::Label::builder()
            .label(ui_text::QR_PROMPT_SECURITY_KEY_HEADING)
            .xalign(0.0)
            .css_classes(["heading"])
            .build();
        let security_key_note = gtk4::Label::builder()
            .label(ui_text::QR_PROMPT_SECURITY_KEY_DESCRIPTION)
            .xalign(0.0)
            .wrap(true)
            .css_classes(["dim-label"])
            .build();
        let security_key_help = gtk4::Box::new(gtk4::Orientation::Vertical, 4);
        security_key_help.set_size_request(CONTENT_WIDTH, -1);
        security_key_help.append(&security_key_heading);
        security_key_help.append(&security_key_note);
        root.append(&overlay);
        let bluetooth_note = gtk4::Label::builder()
            .label(ui_text::QR_PROMPT_BLUETOOTH_REQUIRED)
            .xalign(0.0)
            .wrap(true)
            .css_classes(["dim-label"])
            .build();
        bluetooth_note.set_size_request(CONTENT_WIDTH, -1);
        root.append(&bluetooth_note);
        root.append(&security_key_help);

        let view = Self {
            root,
            qr_code,
            unavailable_card,
            unavailable_message,
            security_key_help,
        };
        view.render(presentation);
        view
    }

    pub(crate) fn widget(&self) -> gtk4::Widget {
        self.root.clone().upcast()
    }

    pub(crate) fn set_qr_accessible_label(&self, label: &str) {
        self.qr_code
            .update_property(&[gtk4::accessible::Property::Label(label)]);
    }

    pub(crate) fn render(&self, presentation: &HybridQrPresentation) {
        self.security_key_help
            .set_visible(presentation.security_key_available);
        self.render_unavailable(presentation.unavailable);
    }

    pub(crate) fn show_unavailable(&self, reason: HybridTransportUnavailableReason) {
        self.render_unavailable(Some(reason));
    }

    fn render_unavailable(&self, reason: Option<HybridTransportUnavailableReason>) {
        self.unavailable_message
            .set_label(keyringd_ui_core::qr_prompt_window::hybrid_unavailable_message(reason));
        self.qr_code
            .set_opacity(if reason.is_some() { 0.25 } else { 1.0 });
        self.unavailable_card.set_visible(reason.is_some());
    }
}
