use gtk4::glib::object::Cast;
use gtk4::prelude::{DrawingAreaExt, DrawingAreaExtManual};

use crate::app::ui::HYBRID_QR_SIZE;

pub(in crate::app::ui) struct QrCodeView;

impl QrCodeView {
    pub(in crate::app::ui) fn render(value: &str) -> gtk4::Widget {
        let qr_code = match qrcode::QrCode::new(value.as_bytes()) {
            Ok(qr_code) => qr_code,
            Err(error) => {
                tracing::warn!(%error, "failed to render Hybrid QR code");
                return gtk4::Label::builder()
                    .label(value)
                    .selectable(true)
                    .wrap(true)
                    .build()
                    .upcast();
            }
        };

        const QUIET_ZONE_MODULES: usize = 4;
        let modules = qr_code.to_colors();
        let qr_width = qr_code.width();
        let drawing_area = gtk4::DrawingArea::builder()
            .accessible_role(gtk4::AccessibleRole::Img)
            .build();
        drawing_area.set_content_width(HYBRID_QR_SIZE);
        drawing_area.set_content_height(HYBRID_QR_SIZE);
        drawing_area.set_draw_func(move |_, context, width, height| {
            let total_modules = qr_width + QUIET_ZONE_MODULES * 2;
            let module_size = (width.min(height) as usize / total_modules).max(1);
            let rendered_size = module_size * total_modules;
            let offset_x = (width as usize - rendered_size) / 2;
            let offset_y = (height as usize - rendered_size) / 2;

            context.set_source_rgb(1.0, 1.0, 1.0);
            let _ = context.paint();
            context.set_source_rgb(0.0, 0.0, 0.0);
            for y in 0..qr_width {
                for x in 0..qr_width {
                    if modules[y * qr_width + x] == qrcode::Color::Dark {
                        context.rectangle(
                            (offset_x + (x + QUIET_ZONE_MODULES) * module_size) as f64,
                            (offset_y + (y + QUIET_ZONE_MODULES) * module_size) as f64,
                            module_size as f64,
                            module_size as f64,
                        );
                    }
                }
            }
            let _ = context.fill();
        });
        drawing_area.upcast()
    }
}
