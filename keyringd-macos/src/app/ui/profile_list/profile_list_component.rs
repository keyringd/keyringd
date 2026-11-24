use keyringd_ui_core::runtime::UiEventTarget;
use keyringd_ui_core::text as ui_text;
use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2::{MainThreadMarker, MainThreadOnly};
use objc2_app_kit::{
    NSAccessibility, NSAutoresizingMaskOptions, NSBorderType, NSBox, NSBoxType, NSColor,
    NSDragOperation, NSFont, NSLayoutConstraint, NSScrollView, NSTableColumn, NSTableView,
    NSTableViewColumnAutoresizingStyle, NSTableViewSelectionHighlightStyle, NSTextField,
    NSTitlePosition, NSToolbar, NSView,
};
use objc2_foundation::{NSArray, NSPoint, NSRect, NSSize, NSString};

use crate::app::ui::Layout;

use super::ProfileListTableDataSource;
use super::ProfilesEmptyStateComponent;
use super::profile_hardware_warning::ProfileHardwareWarning;
use super::{ProfileListEvent, ProfileListProps, ProfileListToolbarDelegate};

pub(crate) struct ProfileListComponent {
    rendered_revision: Option<u64>,
    editing_id: Option<keyringd_core::model::ProfileId>,
    root: Retained<NSView>,
    hardware_warning: ProfileHardwareWarning,
    content: Retained<NSView>,
    content_without_warning: Retained<NSLayoutConstraint>,
    content_below_warning: Retained<NSLayoutConstraint>,
    populated: Retained<NSView>,
    body: Retained<NSView>,
    toolbar: Retained<NSToolbar>,
    _toolbar_delegate: Retained<ProfileListToolbarDelegate>,
    events: UiEventTarget<ProfileListEvent>,
    table_data_source: Option<Retained<ProfileListTableDataSource>>,
    empty_state: Option<ProfilesEmptyStateComponent>,
}

impl ProfileListComponent {
    pub(crate) fn new(events: UiEventTarget<ProfileListEvent>, mtm: MainThreadMarker) -> Self {
        let root = NSView::initWithFrame(
            NSView::alloc(mtm),
            NSRect::new(NSPoint::new(0., 0.), NSSize::new(760., 520.)),
        );
        let toolbar_delegate = ProfileListToolbarDelegate::new(mtm, events.clone());
        let toolbar = NSToolbar::initWithIdentifier(
            NSToolbar::alloc(mtm),
            &NSString::from_str("com.keyringd.profiles-toolbar"),
        );
        toolbar.setAllowsUserCustomization(false);
        toolbar.setAutosavesConfiguration(false);
        toolbar.setDelegate(Some(ProtocolObject::from_ref(&*toolbar_delegate)));
        let content = NSView::new(mtm);
        root.addSubview(&content);
        Layout::prepare(&content);
        content
            .leadingAnchor()
            .constraintEqualToAnchor_constant(&root.leadingAnchor(), 24.0)
            .setActive(true);
        content
            .trailingAnchor()
            .constraintEqualToAnchor_constant(&root.trailingAnchor(), -24.0)
            .setActive(true);
        content
            .bottomAnchor()
            .constraintEqualToAnchor_constant(&root.bottomAnchor(), -24.0)
            .setActive(true);
        let populated = NSView::new(mtm);
        content.addSubview(&populated);
        Layout::pin(&populated, &content, 0.0, 0.0, 0.0, 0.0);
        let introduction = Self::label(
            &populated,
            ui_text::PROFILE_LIST_TITLE,
            32.0,
            462.0,
            560.0,
            30.0,
            22.0,
            true,
            mtm,
        );
        let hardware_warning = ProfileHardwareWarning::new(mtm);
        let banner = hardware_warning.view();
        root.addSubview(banner);
        Layout::prepare(banner);
        banner
            .topAnchor()
            .constraintEqualToAnchor_constant(&root.topAnchor(), 24.0)
            .setActive(true);
        banner
            .leadingAnchor()
            .constraintEqualToAnchor_constant(&root.leadingAnchor(), 24.0)
            .setActive(true);
        banner
            .trailingAnchor()
            .constraintEqualToAnchor_constant(&root.trailingAnchor(), -24.0)
            .setActive(true);
        Layout::prepare(&introduction);
        introduction
            .leadingAnchor()
            .constraintEqualToAnchor(&populated.leadingAnchor())
            .setActive(true);
        introduction
            .trailingAnchor()
            .constraintEqualToAnchor(&populated.trailingAnchor())
            .setActive(true);
        introduction
            .topAnchor()
            .constraintEqualToAnchor(&populated.topAnchor())
            .setActive(true);
        let content_without_warning = content
            .topAnchor()
            .constraintEqualToAnchor(&banner.topAnchor());
        let content_below_warning = content
            .topAnchor()
            .constraintEqualToAnchor_constant(&banner.bottomAnchor(), 16.0);
        content_without_warning.setActive(true);
        Layout::height(&introduction, 30.0);
        let description = Self::label(
            &populated,
            ui_text::PROFILE_LIST_DESCRIPTION,
            32.0,
            438.0,
            560.0,
            20.0,
            13.0,
            false,
            mtm,
        );
        Layout::prepare(&description);
        description
            .topAnchor()
            .constraintEqualToAnchor_constant(&introduction.bottomAnchor(), 2.0)
            .setActive(true);
        description
            .leadingAnchor()
            .constraintEqualToAnchor(&introduction.leadingAnchor())
            .setActive(true);
        description
            .trailingAnchor()
            .constraintEqualToAnchor(&introduction.trailingAnchor())
            .setActive(true);
        Layout::height(&description, 20.0);

        let list_container = NSBox::initWithFrame(
            NSBox::alloc(mtm),
            NSRect::new(NSPoint::new(24.0, 20.0), NSSize::new(712.0, 388.0)),
        );
        list_container.setBoxType(NSBoxType::Primary);
        list_container.setTitlePosition(NSTitlePosition::NoTitle);
        list_container.setContentViewMargins(NSSize::new(0.0, 0.0));
        populated.addSubview(&list_container);
        Layout::prepare(&list_container);
        list_container
            .topAnchor()
            .constraintEqualToAnchor_constant(&description.bottomAnchor(), 18.0)
            .setActive(true);
        list_container
            .leadingAnchor()
            .constraintEqualToAnchor(&populated.leadingAnchor())
            .setActive(true);
        list_container
            .trailingAnchor()
            .constraintEqualToAnchor(&populated.trailingAnchor())
            .setActive(true);
        list_container
            .bottomAnchor()
            .constraintEqualToAnchor(&populated.bottomAnchor())
            .setActive(true);
        let body = NSView::initWithFrame(
            NSView::alloc(mtm),
            NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(712.0, 384.0)),
        );
        list_container.addSubview(&body);
        Layout::pin(&body, &list_container, 1.0, 1.0, 1.0, 1.0);
        Self {
            rendered_revision: None,
            editing_id: None,
            root,
            hardware_warning,
            content,
            content_without_warning,
            content_below_warning,
            populated,
            body,
            toolbar,
            _toolbar_delegate: toolbar_delegate,
            events,
            table_data_source: None,
            empty_state: None,
        }
    }

    pub(crate) fn toolbar(&self) -> &NSToolbar {
        &self.toolbar
    }

    pub(crate) fn render(
        &mut self,
        props: ProfileListProps,
        device_key_error: Option<&str>,
        mtm: MainThreadMarker,
    ) {
        self.hardware_warning.set_message(device_key_error);
        let has_warning = device_key_error.is_some();
        if self.content_below_warning.isActive() != has_warning {
            let (previous, next) = if has_warning {
                (&self.content_without_warning, &self.content_below_warning)
            } else {
                (&self.content_below_warning, &self.content_without_warning)
            };
            previous.setActive(false);
            next.setActive(true);
        }
        for item in self.toolbar.items() {
            item.setEnabled(props.creation_enabled);
        }
        if let Some(empty) = &self.empty_state {
            empty.set_creation_enabled(props.creation_enabled);
        }
        let editing_id = props.editing.as_ref().map(|(id, _)| id.clone());
        if self.rendered_revision == Some(props.revision) && self.editing_id == editing_id {
            // Reconcile the model while preserving AppKit's active field
            // editor, selection and marked text for an in-progress rename.
            if let Some(source) = &self.table_data_source {
                source.set_editing(props.editing);
            }
            return;
        }
        self.rendered_revision = Some(props.revision);
        self.editing_id = editing_id;
        for view in self.body.subviews() {
            view.removeFromSuperview();
        }
        self.table_data_source = None;
        if let Some(empty_state) = self.empty_state.take() {
            empty_state.view().removeFromSuperview();
        }
        let is_empty = props.profiles.is_empty();
        self.populated.setHidden(is_empty);
        self.toolbar.setVisible(!is_empty);
        if is_empty {
            let empty_state = ProfilesEmptyStateComponent::new(self.events.clone(), mtm);
            empty_state.set_creation_enabled(props.creation_enabled);
            self.content.addSubview(empty_state.view());
            Layout::pin(empty_state.view(), &self.content, 0.0, 0.0, 0.0, 0.0);
            self.empty_state = Some(empty_state);
            return;
        }
        let scroll = NSScrollView::initWithFrame(
            NSScrollView::alloc(mtm),
            NSRect::new(NSPoint::new(0.0, 0.0), self.body.bounds().size),
        );
        scroll.setAutoresizingMask(
            NSAutoresizingMaskOptions::ViewWidthSizable
                | NSAutoresizingMaskOptions::ViewHeightSizable,
        );
        scroll.setHasVerticalScroller(true);
        scroll.setAutohidesScrollers(true);
        scroll.setHasHorizontalScroller(false);
        scroll.setBorderType(NSBorderType::NoBorder);
        scroll.setDrawsBackground(false);
        self.body.addSubview(&scroll);
        Layout::pin(&scroll, &self.body, 0.0, 0.0, 0.0, 0.0);
        let table = NSTableView::initWithFrame(
            NSTableView::alloc(mtm),
            NSRect::new(NSPoint::new(0.0, 0.0), self.body.bounds().size),
        );
        table.setAccessibilityIdentifier(Some(&NSString::from_str("profiles.list")));
        table.setHeaderView(None);
        table.setRowHeight(64.0);
        table.setIntercellSpacing(NSSize::new(0.0, 1.0));
        table.setSelectionHighlightStyle(NSTableViewSelectionHighlightStyle::None);
        table.setVerticalMotionCanBeginDrag(true);
        let column = NSTableColumn::initWithIdentifier(
            NSTableColumn::alloc(mtm),
            &NSString::from_str("profile"),
        );
        // The row pins its switch to its own trailing edge, so the column has to
        // be exactly as wide as what the clip view shows: this root still holds
        // its placeholder frame until Auto Layout sizes it, and a legacy
        // scroller takes width from the row that an overlay scroller does not.
        column.setWidth(scroll.contentSize().width);
        table.addTableColumn(&column);
        table.setAutoresizingMask(NSAutoresizingMaskOptions::ViewWidthSizable);
        table.setColumnAutoresizingStyle(
            NSTableViewColumnAutoresizingStyle::UniformColumnAutoresizingStyle,
        );
        let drag_type = NSString::from_str("org.keyringd.profile-row");
        table.registerForDraggedTypes(&NSArray::from_slice(&[&*drag_type]));
        table.setDraggingSourceOperationMask_forLocal(NSDragOperation::Move, true);
        let data_source = ProfileListTableDataSource::new(
            mtm,
            props.profiles,
            props.saving,
            props.editing,
            self.events.clone(),
            drag_type,
        );
        unsafe {
            table.setDataSource(Some(ProtocolObject::from_ref(&*data_source)));
            table.setDelegate(Some(ProtocolObject::from_ref(&*data_source)));
        }
        scroll.setDocumentView(Some(&table));
        self.table_data_source = Some(data_source);
    }

    pub(crate) fn view(&self) -> &NSView {
        &self.root
    }
    #[allow(clippy::too_many_arguments)]
    fn label(
        content: &objc2_app_kit::NSView,
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
        label.setAutoresizingMask(NSAutoresizingMaskOptions::ViewWidthSizable);
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
