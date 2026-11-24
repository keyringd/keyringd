use super::{ProfileSetupActionTarget, ProfileSetupEvent};
use keyringd_ui_core::runtime::UiEventTarget;
use objc2::rc::Retained;
use objc2::{MainThreadMarker, sel};
use objc2_app_kit::{
    NSAccessibility, NSButton, NSColor, NSFont, NSLayoutAttribute, NSStackView, NSTextField,
    NSUserInterfaceLayoutOrientation, NSView,
};
use objc2_foundation::{NSArray, NSString};

pub(crate) struct ProfileSetupControls;
impl ProfileSetupControls {
    pub(crate) fn label(
        text: &str,
        size: f64,
        bold: bool,
        mtm: MainThreadMarker,
    ) -> Retained<NSTextField> {
        let label = NSTextField::wrappingLabelWithString(&NSString::from_str(text), mtm);
        let font = if bold {
            NSFont::boldSystemFontOfSize(size)
        } else {
            NSFont::systemFontOfSize(size)
        };
        label.setFont(Some(&font));
        if !bold {
            label.setTextColor(Some(&NSColor::secondaryLabelColor()));
        }
        label
    }

    pub(crate) fn stack(
        views: &[&NSView],
        orientation: NSUserInterfaceLayoutOrientation,
        mtm: MainThreadMarker,
    ) -> Retained<NSStackView> {
        let stack = NSStackView::stackViewWithViews(&NSArray::from_slice(views), mtm);
        stack.setOrientation(orientation);
        stack.setAlignment(
            if orientation == NSUserInterfaceLayoutOrientation::Vertical {
                NSLayoutAttribute::Leading
            } else {
                NSLayoutAttribute::CenterY
            },
        );
        stack
    }

    pub(crate) fn radio(
        title: &str,
        identifier: &str,
        event: ProfileSetupEvent,
        events: UiEventTarget<ProfileSetupEvent>,
        mtm: MainThreadMarker,
    ) -> (Retained<NSButton>, Retained<ProfileSetupActionTarget>) {
        let target = ProfileSetupActionTarget::new(mtm, events, event);
        let button = unsafe {
            NSButton::radioButtonWithTitle_target_action(
                &NSString::from_str(title),
                Some(&target),
                Some(sel!(performAction:)),
                mtm,
            )
        };
        button.setAccessibilityIdentifier(Some(&NSString::from_str(identifier)));
        (button, target)
    }
}
