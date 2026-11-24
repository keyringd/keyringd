use keyringd_ui_core::text as ui_text;
use windows_reactor::{
    AccessibilityExt, BackgroundExt, Element, HorizontalAlignment, KeyExt, LayoutExt, PaddingExt,
    TextStyleExt, ThemeRef, Thickness, VerticalAlignment, VisualExt, border, button, canvas, grid,
    text_block, vstack,
};

use keyringd_fido_client::OperationId;
use keyringd_ui_core::qr_prompt_window::HybridQrPresentation;
use keyringd_ui_core::runtime::UiEventTarget;

use keyringd_ui_core::key_operation::KeyOperationEvent;

use qrcode::{Color, QrCode};
use windows_canvas::{ColorF, Rect};

const QR_SIZE: f64 = 240.0;

pub(crate) struct PasskeyInteractionView;

impl PasskeyInteractionView {
    pub(crate) fn render(
        presentation: &HybridQrPresentation,
        operation_id: OperationId,
        on_event: UiEventTarget<KeyOperationEvent>,
    ) -> Element {
        let qr = Self::qr_code(
            &presentation.value,
            format!("passkey-qr-{:?}", operation_id),
            QR_SIZE,
        );
        let qr = match presentation.unavailable {
            Some(_) => Self::unavailable_qr_overlay(
                qr,
                presentation.status(),
                QR_SIZE,
                crate::app::ui::PASSKEY_CONTENT_WIDTH,
            ),
            None => qr,
        };
        vstack((
            text_block(presentation.instruction()).wrap(),
            qr,
            text_block(ui_text::QR_PROMPT_BLUETOOTH_REQUIRED)
                .wrap()
                .foreground(ThemeRef::SecondaryText),
            if presentation.security_key_available {
                vstack((
                    text_block(ui_text::QR_PROMPT_SECURITY_KEY_HEADING)
                        .font_size(16.0)
                        .semibold(),
                    text_block(ui_text::QR_PROMPT_SECURITY_KEY_DESCRIPTION)
                        .wrap()
                        .foreground(ThemeRef::SecondaryText),
                    button(keyringd_ui_core::text::QR_PROMPT_USE_SECURITY_KEY)
                        .automation_id("passkey-use-security-key")
                        .with_key(format!("setup-usb-{operation_id:?}"))
                        .on_click(move || {
                            on_event.send(KeyOperationEvent::UseSecurityKeyRequested)
                        }),
                ))
                .spacing(10.0)
                .into()
            } else {
                Element::Empty
            },
        ))
        .spacing(12.0)
        .into()
    }

    fn unavailable_qr_overlay(
        qr: Element,
        message: &'static str,
        qr_size: f64,
        overlay_width: f64,
    ) -> Element {
        let notice = border(
            vstack((
                text_block(ui_text::QR_PROMPT_UNAVAILABLE_TITLE)
                    .font_size(16.0)
                    .semibold()
                    .horizontal_alignment(HorizontalAlignment::Center),
                text_block(message)
                    .wrap()
                    .foreground(ThemeRef::SecondaryText)
                    .horizontal_alignment(HorizontalAlignment::Center),
            ))
            .spacing(8.0),
        )
        .background(ThemeRef::SolidBackground)
        .border_brush(ThemeRef::CardStroke)
        .border_thickness(Thickness::uniform(1.0))
        .corner_radius(8.0)
        .padding(Thickness::uniform(14.0))
        .width(overlay_width)
        .horizontal_alignment(HorizontalAlignment::Center)
        .vertical_alignment(VerticalAlignment::Center);
        grid((border(qr).opacity(0.25), notice))
            .width(overlay_width)
            .height(qr_size)
            .into()
    }

    fn qr_code(uri: &str, key: String, size: f64) -> Element {
        let Ok(code) = QrCode::new(uri.as_bytes()) else {
            return text_block(ui_text::QR_PROMPT_RENDER_FAILED).wrap().into();
        };
        let modules = code.width();
        let colors = code.into_colors();
        canvas(move |draw| {
            draw.clear(ColorF::WHITE);
            let brush = draw.create_solid_brush(ColorF::BLACK)?;
            let scale = draw.width.min(draw.height) / (modules as f32 + 8.0);
            for y in 0..modules {
                for x in 0..modules {
                    if colors[y * modules + x] == Color::Dark {
                        draw.fill_rect(
                            &Rect::from_xywh(
                                (x as f32 + 4.0) * scale,
                                (y as f32 + 4.0) * scale,
                                scale.ceil(),
                                scale.ceil(),
                            ),
                            &brush,
                        );
                    }
                }
            }
            Ok(())
        })
        .automation_id("passkey-hybrid-qr")
        .with_key(key)
        .width(size)
        .height(size)
        .horizontal_alignment(HorizontalAlignment::Center)
        .into()
    }
}
