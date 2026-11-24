use super::CredentialTableDataSource;
use super::{PasskeyCredentialSelection, ProfileSetupEvent};
use crate::app::ui::Layout;
use keyringd_ui_core::text as ui_text;
use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2::{MainThreadMarker, MainThreadOnly};
use objc2_app_kit::{
    NSAccessibility, NSAutoresizingMaskOptions, NSBorderType, NSLayoutConstraint,
    NSLayoutPriorityDefaultHigh, NSScrollView, NSTableColumn, NSTableView,
    NSTableViewColumnAutoresizingStyle, NSTableViewStyle, NSView,
};
use objc2_foundation::{NSPoint, NSRect, NSSize, NSString};

use keyringd_ui_core::runtime::UiEventTarget;

pub(crate) struct PasskeySelectionPage {
    container: Retained<NSScrollView>,
    list_height: Retained<NSLayoutConstraint>,
    data_source: Retained<CredentialTableDataSource>,
}

impl PasskeySelectionPage {
    pub(crate) fn new(
        parent: &NSView,
        events: UiEventTarget<ProfileSetupEvent>,
        mtm: MainThreadMarker,
    ) -> Self {
        let container = NSScrollView::new(mtm);
        container.setHasVerticalScroller(true);
        container.setAutohidesScrollers(true);
        container.setHasHorizontalScroller(false);
        container.setBorderType(NSBorderType::BezelBorder);
        parent.addSubview(&container);
        Layout::pin_top(&container, parent);
        container
            .bottomAnchor()
            .constraintLessThanOrEqualToAnchor(&parent.bottomAnchor())
            .setActive(true);
        // Size short lists to their contents; longer lists scroll in the available space.
        let list_height = container.heightAnchor().constraintEqualToConstant(0.0);
        list_height.setPriority(NSLayoutPriorityDefaultHigh);
        list_height.setActive(true);

        let table = NSTableView::initWithFrame(
            NSTableView::alloc(mtm),
            NSRect::new(NSPoint::ZERO, NSSize::new(368.0, 88.0)),
        );
        table.setAccessibilityIdentifier(Some(&NSString::from_str(
            "profile-setup.credential-selection",
        )));
        table.setAccessibilityLabel(Some(&NSString::from_str(ui_text::PROFILE_SETUP_PASSKEYS)));
        table.setHeaderView(None);
        table.setStyle(NSTableViewStyle::Inset);
        table.setRowHeight(44.0);
        table.setIntercellSpacing(NSSize::new(0.0, 0.0));
        table.setAllowsMultipleSelection(false);
        table.setAllowsEmptySelection(false);
        table.setAllowsColumnReordering(false);
        let column = NSTableColumn::initWithIdentifier(
            NSTableColumn::alloc(mtm),
            &NSString::from_str("passkey"),
        );
        column.setEditable(false);
        column.setWidth(368.0);
        table.addTableColumn(&column);
        table.setAutoresizingMask(NSAutoresizingMaskOptions::ViewWidthSizable);
        table.setColumnAutoresizingStyle(
            NSTableViewColumnAutoresizingStyle::UniformColumnAutoresizingStyle,
        );
        let data_source = CredentialTableDataSource::new(table.clone(), events, mtm);
        unsafe {
            table.setDataSource(Some(ProtocolObject::from_ref(&*data_source)));
            table.setDelegate(Some(ProtocolObject::from_ref(&*data_source)));
        }
        container.setDocumentView(Some(&table));
        Self {
            container,
            list_height,
            data_source,
        }
    }

    pub(crate) fn view(&self) -> &NSView {
        &self.container
    }

    pub(crate) fn render(&self, selection: &PasskeyCredentialSelection) {
        self.list_height
            .setConstant(selection.candidates().len() as f64 * 44.0 + 16.0);
        self.data_source.render(selection);
    }
}
