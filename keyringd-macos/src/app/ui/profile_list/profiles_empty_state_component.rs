use keyringd_ui_core::profile_list::PROFILES_EMPTY_KEY_ICON;
use keyringd_ui_core::runtime::UiEventTarget;
use keyringd_ui_core::text as ui_text;
use objc2::AnyThread;
use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2::sel;
use objc2_app_kit::{
    NSAccessibility, NSButton, NSCellImagePosition, NSImage, NSImageScaling, NSImageView,
    NSLayoutAttribute, NSTextAlignment, NSUserInterfaceLayoutOrientation, NSView,
};
use objc2_foundation::{NSData, NSString};

use super::ProfileListEvent;
use crate::app::ui::{ControlActionTarget, Layout};

pub(crate) struct ProfilesEmptyStateComponent {
    root: Retained<NSView>,
    add: Retained<NSButton>,
    _add_target: Retained<ControlActionTarget>,
}

impl ProfilesEmptyStateComponent {
    pub(crate) fn new(events: UiEventTarget<ProfileListEvent>, mtm: MainThreadMarker) -> Self {
        let root = NSView::new(mtm);
        let group = Layout::stack(&[], NSUserInterfaceLayoutOrientation::Vertical, 12.0, mtm);
        group.setAlignment(NSLayoutAttribute::CenterX);
        root.addSubview(&group);
        Layout::prepare(&group);
        Layout::center_x(&group, &root);
        Layout::center_y(&group, &root, 0.0);

        let image = NSImage::initWithData(
            NSImage::alloc(),
            &NSData::with_bytes(PROFILES_EMPTY_KEY_ICON),
        )
        .expect("the embedded key icon is a valid PNG");
        image.setAccessibilityDescription(Some(&NSString::from_str(
            ui_text::PROFILE_LIST_KEY_ACCESSIBILITY,
        )));
        let icon = NSImageView::imageViewWithImage(&image, mtm);
        icon.setImageScaling(NSImageScaling::ScaleProportionallyUpOrDown);
        group.addArrangedSubview(&icon);
        Layout::prepare(&icon);
        Layout::width(&icon, 112.0);
        Layout::height(&icon, 112.0);
        group.setCustomSpacing_afterView(24.0, &icon);

        let title = Layout::label(ui_text::PROFILE_LIST_EMPTY_TITLE, 24.0, true, mtm);
        title.setAlignment(NSTextAlignment::Center);
        group.addArrangedSubview(&title);
        group.setCustomSpacing_afterView(8.0, &title);

        let detail = Layout::label(ui_text::PROFILE_LIST_EMPTY_DESCRIPTION, 13.0, false, mtm);
        detail.setAlignment(NSTextAlignment::Center);
        detail.setPreferredMaxLayoutWidth(448.0);
        group.addArrangedSubview(&detail);
        Layout::prepare(&detail);
        Layout::width(&detail, 448.0);
        group.setCustomSpacing_afterView(24.0, &detail);

        let add_target = ControlActionTarget::new(mtm, events, ProfileListEvent::AddRequested);
        let add = unsafe {
            NSButton::buttonWithTitle_target_action(
                &NSString::from_str(ui_text::PROFILE_LIST_ADD),
                Some(&add_target),
                Some(sel!(performAction:)),
                mtm,
            )
        };
        let plus = NSImage::imageWithSystemSymbolName_accessibilityDescription(
            &NSString::from_str("plus"),
            None,
        );
        add.setImage(plus.as_deref());
        add.setImagePosition(NSCellImagePosition::ImageLeft);
        add.setAccessibilityIdentifier(Some(&NSString::from_str("profiles.empty.add")));
        group.addArrangedSubview(&add);

        Self {
            root,
            add,
            _add_target: add_target,
        }
    }

    pub(crate) fn set_creation_enabled(&self, enabled: bool) {
        self.add.setEnabled(enabled);
    }

    pub(crate) fn view(&self) -> &NSView {
        &self.root
    }
}
