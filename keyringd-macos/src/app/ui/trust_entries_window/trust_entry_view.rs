use crate::app::ui::Layout;
use keyringd_ui_core::runtime::UiEventSender;
use objc2::rc::Retained;
use objc2::{MainThreadMarker, sel};
use objc2_app_kit::{
    NSAccessibility, NSBezelStyle, NSButton, NSFont, NSFontWeightRegular, NSImage,
    NSImageSymbolConfiguration, NSImageSymbolScale, NSLineBreakMode, NSTextField,
    NSUserInterfaceLayoutOrientation, NSView,
};
use objc2_foundation::NSString;

use super::TrustEntriesEvent;
use super::TrustEntriesWindowActionTarget;
use super::TrustEntryViewProps;

pub(super) struct TrustEntryView {
    root: Retained<NSView>,
    _copy_target: Retained<TrustEntriesWindowActionTarget>,
}

impl TrustEntryView {
    pub(super) fn render(
        props: TrustEntryViewProps,
        events: UiEventSender<TrustEntriesEvent>,
        button_identifier: &str,
        mtm: MainThreadMarker,
    ) -> Self {
        let root = NSView::new(mtm);
        let title = Layout::label(props.kind.title(), 12.0, true, mtm);
        let subtitle = Layout::label(props.description, 11.0, false, mtm);
        let field = NSTextField::new(mtm);
        field.setStringValue(&NSString::from_str(&props.value));
        field.setAccessibilityIdentifier(Some(&NSString::from_str(&format!(
            "{button_identifier}.value"
        ))));
        field.setEditable(false);
        field.setSelectable(true);
        field.setUsesSingleLineMode(true);
        field.setLineBreakMode(NSLineBreakMode::ByClipping);
        if let Some(cell) = field.cell() {
            cell.setScrollable(true);
        }
        field.setFont(Some(&NSFont::monospacedSystemFontOfSize_weight(11.0, 0.0)));
        Layout::flexible_width(&field);
        let target = TrustEntriesWindowActionTarget::new(
            mtm,
            events,
            TrustEntriesEvent::CopyRequested(props.kind),
        );
        let image = NSImage::imageWithSystemSymbolName_accessibilityDescription(
            &NSString::from_str("doc.on.doc"),
            Some(&NSString::from_str(props.kind.copy_label())),
        )
        .expect("the standard doc.on.doc symbol should be available on supported macOS versions");
        let button = unsafe {
            NSButton::buttonWithImage_target_action(
                &image,
                Some(&target),
                Some(sel!(performAction:)),
                mtm,
            )
        };
        button.setBezelStyle(NSBezelStyle::AccessoryBarAction);
        let symbol = NSImageSymbolConfiguration::configurationWithPointSize_weight_scale(
            14.0,
            unsafe { NSFontWeightRegular },
            NSImageSymbolScale::Medium,
        );
        button.setSymbolConfiguration(Some(&symbol));
        button.setToolTip(Some(&NSString::from_str(props.kind.copy_label())));
        button.setAccessibilityIdentifier(Some(&NSString::from_str(button_identifier)));
        let row = Layout::stack(
            &[&field, &button],
            NSUserInterfaceLayoutOrientation::Horizontal,
            8.0,
            mtm,
        );
        let stack = Layout::stack(
            &[&title, &subtitle, &row],
            NSUserInterfaceLayoutOrientation::Vertical,
            8.0,
            mtm,
        );
        root.addSubview(&stack);
        Layout::pin(&stack, &root, 0.0, 0.0, 0.0, 0.0);
        Self {
            root,
            _copy_target: target,
        }
    }

    pub(super) fn view(&self) -> &NSView {
        &self.root
    }
}
