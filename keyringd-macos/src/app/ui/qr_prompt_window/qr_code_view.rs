use objc2::AnyThread;
use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2_app_kit::{NSImage, NSImageScaling, NSImageView};
use objc2_foundation::NSData;
use qrcode::QrCode;

pub(crate) struct QrCodeView {
    view: Retained<NSImageView>,
}

impl QrCodeView {
    /// AppKit points, independent of the display's backing scale.
    pub(crate) const SIZE: f64 = 240.0;

    pub(crate) fn new(value: &str, mtm: MainThreadMarker) -> anyhow::Result<Self> {
        let pixels = QrCode::new(value.as_bytes())?
            .render::<image::Luma<u8>>()
            .quiet_zone(true)
            .min_dimensions(Self::SIZE as u32, Self::SIZE as u32)
            .max_dimensions(Self::SIZE as u32, Self::SIZE as u32)
            .build();
        let mut png = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageLuma8(pixels).write_to(&mut png, image::ImageFormat::Png)?;
        let png = png.into_inner();
        let data = unsafe { NSData::dataWithBytes_length(png.as_ptr().cast(), png.len()) };
        let image = NSImage::initWithData(NSImage::alloc(), &data)
            .ok_or_else(|| anyhow::anyhow!("AppKit could not decode the generated QR image"))?;
        let view = NSImageView::imageViewWithImage(&image, mtm);
        view.setImageScaling(NSImageScaling::ScaleProportionallyUpOrDown);
        Ok(Self { view })
    }

    pub(crate) fn view(&self) -> &NSImageView {
        &self.view
    }
}
