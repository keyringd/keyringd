use keyringd_ui_core::text as ui_text;
use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2::sel;
use objc2::{MainThreadMarker, MainThreadOnly};
use objc2_app_kit::{
    NSAccessibility, NSBox, NSBoxType, NSColor, NSControlStateValueOff, NSControlStateValueOn,
    NSFont, NSPopUpButton, NSSwitch, NSTextField, NSUserInterfaceItemIdentification, NSView,
};
use objc2_foundation::{NSPoint, NSRect, NSSize, NSString};

use keyringd_ui_core::profile_list_item::ProfileActions;

use keyringd_ui_core::runtime::UiEventTarget;

use crate::app::ui::Layout;

use crate::app::ui::ControlActionTarget;

use super::ProfileListItemEvent;
use super::ProfileListItemProps;
use super::ProfileNameFieldDelegate;

pub(crate) struct ProfileListItemComponent {
    root: Retained<NSView>,
    _targets: Vec<Retained<ControlActionTarget>>,
    _name_delegate: Option<Retained<ProfileNameFieldDelegate>>,
}

impl ProfileListItemComponent {
    pub(crate) fn build(
        props: ProfileListItemProps,
        events: UiEventTarget<ProfileListItemEvent>,
        position: (usize, usize),
        frame: NSRect,
        mtm: MainThreadMarker,
    ) -> Self {
        let root = NSView::initWithFrame(NSView::alloc(mtm), frame);
        let subtitle = props.subtitle();
        let row_actions = ProfileActions::for_profile(
            &props.profile,
            ui_text::DEVICE_KEY_PROTECTION_RECONFIGURE,
            position,
        );
        let saving = props.saving;
        let profile = props.profile;
        let profile_name = profile.display_name().to_owned();
        let drag_handle = Self::label(&root, "≡", 8.0, 22.0, 18.0, 24.0, 17.0, false, mtm);
        drag_handle.setTextColor(Some(&NSColor::tertiaryLabelColor()));
        drag_handle.setToolTip(Some(&NSString::from_str(ui_text::PROFILE_LIST_ITEM_DRAG)));
        let (title, name_delegate) = if let Some(editing_name) = props.editing_name {
            let field = NSTextField::new(mtm);
            field.setEnabled(!saving);
            field.setStringValue(&NSString::from_str(&editing_name));
            field.setFont(Some(&NSFont::systemFontOfSize(15.0)));
            field.setFrame(NSRect::new(
                NSPoint::new(34.0, 34.0),
                NSSize::new(360.0, 24.0),
            ));
            let delegate = ProfileNameFieldDelegate::new(
                mtm,
                profile.id().clone(),
                field.clone(),
                events.clone(),
            );
            unsafe {
                field.setDelegate(Some(ProtocolObject::from_ref(&*delegate)));
            }
            Layout::focus_after_attachment(&field, mtm);
            root.addSubview(&field);
            (field, Some(delegate))
        } else {
            (
                Self::label(
                    &root,
                    profile.display_name(),
                    34.0,
                    34.0,
                    360.0,
                    24.0,
                    15.0,
                    true,
                    mtm,
                ),
                None,
            )
        };
        let profile_id = profile.id().as_str();
        // The same identifier names the label and, while renaming, the text
        // field that replaces it, so a test drives the row the user sees.
        title.setAccessibilityIdentifier(Some(&NSString::from_str(&format!(
            "profiles.row.{profile_id}.name"
        ))));
        if !profile.is_enabled() {
            title.setTextColor(Some(&NSColor::secondaryLabelColor()));
        }
        let detail = Self::label(&root, &subtitle, 34.0, 10.0, 440.0, 20.0, 12.0, false, mtm);
        let mut targets = Vec::new();
        let enabled = row_actions
            .enabled_changed(!profile.is_enabled())
            .expect("the switch toggles the current profile state");
        let target = ControlActionTarget::new(mtm, events.clone(), enabled);
        let control = NSSwitch::initWithFrame(
            NSSwitch::alloc(mtm),
            NSRect::new(NSPoint::new(626.0, 20.0), NSSize::new(48.0, 24.0)),
        );
        control.setEnabled(!saving);
        control.setState(if profile.is_enabled() {
            NSControlStateValueOn
        } else {
            NSControlStateValueOff
        });
        control.setToolTip(Some(&NSString::from_str(
            &keyringd_ui_core::profile_list_item::enable_profile(&profile_name),
        )));
        control.setAccessibilityIdentifier(Some(&NSString::from_str(&format!(
            "profiles.row.{profile_id}.enabled"
        ))));
        unsafe {
            control.setTarget(Some(&target));
            control.setAction(Some(sel!(performAction:)));
        }
        root.addSubview(&control);
        targets.push(target);

        let popup = NSPopUpButton::initWithFrame_pullsDown(
            NSPopUpButton::alloc(mtm),
            NSRect::new(NSPoint::new(566.0, 18.0), NSSize::new(44.0, 28.0)),
            true,
        );
        popup.setEnabled(!saving);
        popup.addItemWithTitle(&NSString::from_str("•••"));
        popup.setToolTip(Some(&NSString::from_str(
            &keyringd_ui_core::profile_list_item::actions_for(&profile_name),
        )));
        popup.setAccessibilityIdentifier(Some(&NSString::from_str(&format!(
            "profiles.row.{profile_id}.actions"
        ))));
        popup.setAutoenablesItems(false);
        for action in &row_actions.items {
            popup.addItemWithTitle(&NSString::from_str(action.label));
            let target = action
                .activate()
                .map(|event| ControlActionTarget::new(mtm, events.clone(), event));
            if let Some(item) = popup.lastItem() {
                item.setIdentifier(Some(&NSString::from_str(action.identifier)));
                item.setEnabled(action.enabled);
                if let Some(target) = &target {
                    unsafe {
                        item.setTarget(Some(target));
                        item.setAction(Some(sel!(performAction:)));
                    }
                }
            }
            if let Some(target) = target {
                targets.push(target);
            }
        }
        root.addSubview(&popup);
        let separator = NSBox::initWithFrame(
            NSBox::alloc(mtm),
            NSRect::new(
                NSPoint::new(12.0, 0.0),
                NSSize::new(frame.size.width - 24.0, 1.0),
            ),
        );
        separator.setBoxType(NSBoxType::Separator);
        root.addSubview(&separator);

        Layout::prepare(&title);
        title
            .leadingAnchor()
            .constraintEqualToAnchor_constant(&root.leadingAnchor(), 34.0)
            .setActive(true);
        title
            .topAnchor()
            .constraintEqualToAnchor_constant(&root.topAnchor(), 7.0)
            .setActive(true);
        title
            .trailingAnchor()
            .constraintLessThanOrEqualToAnchor_constant(&control.leadingAnchor(), -16.0)
            .setActive(true);
        Layout::height(&title, 24.0);

        Layout::prepare(&detail);
        detail
            .leadingAnchor()
            .constraintEqualToAnchor(&title.leadingAnchor())
            .setActive(true);
        detail
            .topAnchor()
            .constraintEqualToAnchor_constant(&title.bottomAnchor(), 4.0)
            .setActive(true);
        detail
            .trailingAnchor()
            .constraintLessThanOrEqualToAnchor_constant(&control.leadingAnchor(), -16.0)
            .setActive(true);
        Layout::height(&detail, 20.0);

        Layout::prepare(&control);
        control
            .trailingAnchor()
            .constraintEqualToAnchor_constant(&popup.leadingAnchor(), -12.0)
            .setActive(true);
        control
            .centerYAnchor()
            .constraintEqualToAnchor(&root.centerYAnchor())
            .setActive(true);
        Layout::width(&control, 48.0);
        Layout::height(&control, 24.0);

        Layout::prepare(&popup);
        popup
            .trailingAnchor()
            .constraintEqualToAnchor_constant(&root.trailingAnchor(), -12.0)
            .setActive(true);
        popup
            .centerYAnchor()
            .constraintEqualToAnchor(&root.centerYAnchor())
            .setActive(true);
        Layout::width(&popup, 44.0);
        Layout::height(&popup, 28.0);

        Layout::prepare(&separator);
        separator
            .leadingAnchor()
            .constraintEqualToAnchor_constant(&root.leadingAnchor(), 12.0)
            .setActive(true);
        separator
            .trailingAnchor()
            .constraintEqualToAnchor_constant(&root.trailingAnchor(), -12.0)
            .setActive(true);
        separator
            .bottomAnchor()
            .constraintEqualToAnchor(&root.bottomAnchor())
            .setActive(true);
        Layout::height(&separator, 1.0);
        Self {
            root,
            _targets: targets,
            _name_delegate: name_delegate,
        }
    }

    pub(crate) fn view(&self) -> &NSView {
        &self.root
    }

    #[allow(clippy::too_many_arguments)]
    fn label(
        content: &NSView,
        text: &str,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        size: f64,
        bold: bool,
        mtm: MainThreadMarker,
    ) -> Retained<NSTextField> {
        let label = NSTextField::wrappingLabelWithString(&NSString::from_str(text), mtm);
        label.setFrame(NSRect::new(NSPoint::new(x, y), NSSize::new(width, height)));
        let font = if bold {
            NSFont::boldSystemFontOfSize(size)
        } else {
            NSFont::systemFontOfSize(size)
        };
        label.setFont(Some(&font));
        if !bold {
            label.setTextColor(Some(&NSColor::secondaryLabelColor()));
        }
        content.addSubview(&label);
        label
    }
}
