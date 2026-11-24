use keyringd_fido_client::HybridTransportUnavailableReason;
use keyringd_ui_core::qr_prompt_window::HybridQrPresentation;
use keyringd_ui_core::text as ui_text;
use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2_app_kit::{
    NSAccessibility, NSBox, NSBoxType, NSColor, NSFont, NSLayoutAttribute, NSStackView,
    NSTextField, NSTitlePosition, NSUserInterfaceLayoutOrientation, NSView,
};
use objc2_foundation::{NSArray, NSString};

use super::QrCodeView;
use crate::app::ui::Layout;

pub(crate) struct HybridQrView {
    root: Retained<NSStackView>,
    qr: Retained<NSView>,
    unavailable: Retained<NSTextField>,
    unavailable_card: Retained<NSBox>,
    security_key_help: Retained<NSStackView>,
}

impl HybridQrView {
    pub(crate) fn new(presentation: &HybridQrPresentation, mtm: MainThreadMarker) -> Self {
        let qr = NSView::new(mtm);
        Layout::width(&qr, QrCodeView::SIZE);
        Layout::height(&qr, QrCodeView::SIZE);
        match QrCodeView::new(&presentation.value, mtm) {
            Ok(image) => {
                image
                    .view()
                    .setAccessibilityIdentifier(Some(&NSString::from_str("key-operation.qr")));
                qr.addSubview(image.view());
                Layout::pin(image.view(), &qr, 0.0, 0.0, 0.0, 0.0);
            }
            Err(error) => {
                tracing::warn!(%error, "could not render Hybrid QR code");
                let message = Self::label(ui_text::QR_PROMPT_RENDER_FAILED, 13.0, mtm);
                qr.addSubview(&message);
                Layout::pin(&message, &qr, 0.0, 0.0, 0.0, 0.0);
            }
        }
        let overlay = NSView::new(mtm);
        Layout::height(&overlay, QrCodeView::SIZE);
        overlay.addSubview(&qr);
        Layout::prepare(&qr);
        qr.centerXAnchor()
            .constraintEqualToAnchor(&overlay.centerXAnchor())
            .setActive(true);
        qr.topAnchor()
            .constraintEqualToAnchor(&overlay.topAnchor())
            .setActive(true);
        // The native card overlays the dimmed QR without moving the controls.
        let unavailable_card = NSBox::new(mtm);
        unavailable_card.setBoxType(NSBoxType::Custom);
        unavailable_card.setTitlePosition(NSTitlePosition::NoTitle);
        unavailable_card.setFillColor(&NSColor::windowBackgroundColor());
        unavailable_card.setBorderColor(&NSColor::separatorColor());
        unavailable_card.setCornerRadius(8.0);
        overlay.addSubview(&unavailable_card);
        Layout::prepare(&unavailable_card);
        unavailable_card
            .centerXAnchor()
            .constraintEqualToAnchor(&overlay.centerXAnchor())
            .setActive(true);
        unavailable_card
            .centerYAnchor()
            .constraintEqualToAnchor(&overlay.centerYAnchor())
            .setActive(true);
        Layout::width(&unavailable_card, 336.0);
        let unavailable_title = Self::label(ui_text::QR_PROMPT_UNAVAILABLE_TITLE, 13.0, mtm);
        unavailable_title.setFont(Some(&NSFont::boldSystemFontOfSize(13.0)));
        let unavailable = Self::label("", 13.0, mtm);
        unavailable.setAccessibilityIdentifier(Some(&NSString::from_str(
            "key-operation.hybrid-unavailable",
        )));
        let notice = NSStackView::stackViewWithViews(
            &NSArray::<NSView>::from_slice(&[&*unavailable_title, &*unavailable]),
            mtm,
        );
        notice.setOrientation(NSUserInterfaceLayoutOrientation::Vertical);
        notice.setAlignment(NSLayoutAttribute::CenterX);
        notice.setSpacing(8.0);
        unavailable_card.addSubview(&notice);
        Layout::pin(&notice, &unavailable_card, 14.0, 14.0, 14.0, 14.0);
        for label in [&unavailable_title, &unavailable] {
            Layout::width(label, 304.0);
        }
        unavailable_card.setHidden(true);
        let security_key_title = Self::label(ui_text::QR_PROMPT_SECURITY_KEY_HEADING, 13.0, mtm);
        security_key_title.setAlignment(objc2_app_kit::NSTextAlignment::Left);
        security_key_title.setFont(Some(&NSFont::boldSystemFontOfSize(13.0)));
        let security_key_note = Self::label(ui_text::QR_PROMPT_SECURITY_KEY_DESCRIPTION, 12.0, mtm);
        security_key_note.setAlignment(objc2_app_kit::NSTextAlignment::Left);
        let security_key_help = NSStackView::stackViewWithViews(
            &NSArray::<NSView>::from_slice(&[&*security_key_title, &*security_key_note]),
            mtm,
        );
        security_key_help.setOrientation(NSUserInterfaceLayoutOrientation::Vertical);
        security_key_help.setAlignment(NSLayoutAttribute::Leading);
        security_key_help.setSpacing(4.0);
        let bluetooth_note = Self::label(ui_text::QR_PROMPT_BLUETOOTH_REQUIRED, 13.0, mtm);
        bluetooth_note.setAlignment(objc2_app_kit::NSTextAlignment::Left);
        bluetooth_note
            .setAccessibilityIdentifier(Some(&NSString::from_str("key-operation.bluetooth-note")));
        let root = NSStackView::stackViewWithViews(
            &NSArray::<NSView>::from_slice(&[&*overlay, &*bluetooth_note, &*security_key_help]),
            mtm,
        );
        root.setOrientation(NSUserInterfaceLayoutOrientation::Vertical);
        root.setAlignment(NSLayoutAttribute::Leading);
        root.setSpacing(12.0);
        for view in [&*overlay as &NSView, &*bluetooth_note, &*security_key_help] {
            Layout::prepare(view);
            view.widthAnchor()
                .constraintEqualToAnchor(&root.widthAnchor())
                .setActive(true);
        }
        let view = Self {
            root,
            qr,
            unavailable,
            unavailable_card,
            security_key_help,
        };
        view.render(presentation);
        view
    }

    pub(crate) fn view(&self) -> &NSView {
        &self.root
    }

    pub(crate) fn render(&self, presentation: &HybridQrPresentation) {
        self.security_key_help
            .setHidden(!presentation.security_key_available);
        self.render_unavailable(presentation.unavailable);
    }

    pub(crate) fn show_unavailable(&self, reason: HybridTransportUnavailableReason) {
        self.render_unavailable(Some(reason));
    }

    fn render_unavailable(&self, reason: Option<HybridTransportUnavailableReason>) {
        self.qr
            .setAlphaValue(if reason.is_some() { 0.25 } else { 1.0 });
        self.unavailable_card.setHidden(reason.is_none());
        let message = keyringd_ui_core::qr_prompt_window::hybrid_unavailable_message(reason);
        self.unavailable
            .setStringValue(&NSString::from_str(message));
    }
}

impl HybridQrView {
    fn label(text: &str, size: f64, mtm: MainThreadMarker) -> Retained<NSTextField> {
        let label = NSTextField::wrappingLabelWithString(&NSString::from_str(text), mtm);
        label.setAlignment(objc2_app_kit::NSTextAlignment::Center);
        label.setFont(Some(&NSFont::systemFontOfSize(size)));
        label.setTextColor(Some(&NSColor::secondaryLabelColor()));
        label
    }
}
