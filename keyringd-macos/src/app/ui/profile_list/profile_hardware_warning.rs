use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2_app_kit::{
    NSAccessibility, NSBox, NSBoxType, NSColor, NSImage, NSImageView, NSTextField, NSTitlePosition,
    NSUserInterfaceLayoutOrientation, NSView,
};
use objc2_foundation::{NSSize, NSString};

use crate::app::ui::Layout;

pub(super) struct ProfileHardwareWarning {
    root: Retained<NSBox>,
    message: Retained<NSTextField>,
}

impl ProfileHardwareWarning {
    pub(super) fn new(mtm: MainThreadMarker) -> Self {
        let message = Layout::label("", 13.0, false, mtm);
        message.setTextColor(Some(&NSColor::labelColor()));
        message.setAccessibilityIdentifier(Some(&NSString::from_str("profiles.hardware-warning")));
        Layout::flexible_width(&message);

        let icon = NSImage::imageWithSystemSymbolName_accessibilityDescription(
            &NSString::from_str("exclamationmark.triangle.fill"),
            None,
        )
        .map(|image| NSImageView::imageViewWithImage(&image, mtm));
        let mut views: Vec<&NSView> = Vec::new();
        if let Some(icon) = &icon {
            icon.setContentTintColor(Some(&NSColor::systemOrangeColor()));
            Layout::prepare(icon);
            Layout::width(icon, 18.0);
            Layout::height(icon, 18.0);
            views.push(icon);
        }
        views.push(&message);
        let row = Layout::stack(
            &views,
            NSUserInterfaceLayoutOrientation::Horizontal,
            12.0,
            mtm,
        );
        let root = NSBox::new(mtm);
        root.setBoxType(NSBoxType::Primary);
        root.setTitlePosition(NSTitlePosition::NoTitle);
        root.setContentViewMargins(NSSize::new(0.0, 0.0));
        root.addSubview(&row);
        Layout::pin(&row, &root, 14.0, 16.0, 14.0, 16.0);
        root.setHidden(true);
        Self { root, message }
    }

    pub(super) fn view(&self) -> &NSView {
        &self.root
    }

    pub(super) fn set_message(&self, message: Option<&str>) {
        self.message
            .setStringValue(&NSString::from_str(message.unwrap_or_default()));
        self.root.setHidden(message.is_none());
    }
}
