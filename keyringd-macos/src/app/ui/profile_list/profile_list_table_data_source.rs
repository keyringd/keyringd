use keyringd_ui_core::runtime::UiEventTarget;
use std::cell::RefCell;

use keyringd_core::model::{Profile, ProfileId};
use objc2::rc::Retained;
use objc2::runtime::{NSObjectProtocol, ProtocolObject};
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, Message, define_class, msg_send};
use objc2_app_kit::{
    NSControlTextEditingDelegate, NSDragOperation, NSDraggingInfo, NSPasteboardItem,
    NSPasteboardWriting, NSTableColumn, NSTableView, NSTableViewDataSource, NSTableViewDelegate,
    NSTableViewDropOperation, NSTextFieldDelegate, NSView,
};
use objc2_foundation::{NSInteger, NSObject, NSString};

use crate::app::ui::profile_list_item::ProfileListItemComponent;
use crate::app::ui::profile_list_item::ProfileListItemEvent;
use crate::app::ui::profile_list_item::ProfileListItemProps;

use super::ProfileListEvent;

pub(super) struct ProfileListTableDataSourceIvars {
    profiles: RefCell<Vec<Profile>>,
    saving: bool,
    editing: RefCell<Option<(ProfileId, String)>>,
    events: UiEventTarget<ProfileListEvent>,
    items: RefCell<Vec<ProfileListItemComponent>>,
    drag_type: Retained<NSString>,
}

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "KeyringdProfileListTableDataSource"]
    #[ivars = ProfileListTableDataSourceIvars]
    pub(super) struct ProfileListTableDataSource;

    unsafe impl NSObjectProtocol for ProfileListTableDataSource {}
    unsafe impl NSControlTextEditingDelegate for ProfileListTableDataSource {}
    unsafe impl NSTextFieldDelegate for ProfileListTableDataSource {}

    unsafe impl NSTableViewDelegate for ProfileListTableDataSource {
        #[unsafe(method_id(tableView:viewForTableColumn:row:))]
        fn table_view_for_row(
            &self,
            table: &NSTableView,
            _column: Option<&NSTableColumn>,
            row: NSInteger,
        ) -> Option<Retained<NSView>> {
            self.row_view(table, row)
        }

        #[unsafe(method(tableView:heightOfRow:))]
        fn table_view_height_of_row(&self, _table: &NSTableView, _row: NSInteger) -> f64 {
            64.0
        }
    }

    unsafe impl NSTableViewDataSource for ProfileListTableDataSource {
        #[unsafe(method(numberOfRowsInTableView:))]
        fn number_of_rows(&self, _table: &NSTableView) -> NSInteger {
            NSInteger::try_from(self.ivars().profiles.borrow().len()).unwrap_or(NSInteger::MAX)
        }

        #[unsafe(method_id(tableView:pasteboardWriterForRow:))]
        fn pasteboard_writer_for_row(
            &self,
            _table: &NSTableView,
            row: NSInteger,
        ) -> Option<Retained<ProtocolObject<dyn NSPasteboardWriting>>> {
            self.pasteboard_writer(row)
        }

        #[unsafe(method(tableView:validateDrop:proposedRow:proposedDropOperation:))]
        fn validate_drop(
            &self,
            table: &NSTableView,
            info: &ProtocolObject<dyn NSDraggingInfo>,
            row: NSInteger,
            _operation: NSTableViewDropOperation,
        ) -> NSDragOperation {
            if self.ivars().saving {
                return NSDragOperation::None;
            }
            let Some(id) = info
                .draggingPasteboard()
                .stringForType(&self.ivars().drag_type)
            else {
                return NSDragOperation::None;
            };
            if !self
                .ivars()
                .profiles
                .borrow()
                .iter()
                .any(|profile| profile.id().as_str() == id.to_string())
            {
                return NSDragOperation::None;
            }
            table.setDropRow_dropOperation(row, NSTableViewDropOperation::Above);
            NSDragOperation::Move
        }

        #[unsafe(method(tableView:acceptDrop:row:dropOperation:))]
        fn accept_drop(
            &self,
            _table: &NSTableView,
            info: &ProtocolObject<dyn NSDraggingInfo>,
            row: NSInteger,
            _operation: NSTableViewDropOperation,
        ) -> bool {
            if self.ivars().saving {
                return false.into();
            }
            let Some(dragged_id) = info
                .draggingPasteboard()
                .stringForType(&self.ivars().drag_type)
                .map(|value| value.to_string())
            else {
                return false.into();
            };
            let profiles = self.ivars().profiles.borrow();
            let Ok(insertion) = usize::try_from(row) else {
                return false.into();
            };
            let original_ids = profiles
                .iter()
                .map(|profile| profile.id().clone())
                .collect::<Vec<ProfileId>>();
            let Some(ids) = keyringd_ui_core::profile_list::reorder_profile(
                &original_ids,
                &dragged_id,
                insertion,
            ) else {
                return false.into();
            };
            let unchanged = ids == original_ids;
            drop(profiles);
            if !unchanged {
                let event =
                    ProfileListEvent::ProfileAction(Box::new(ProfileListItemEvent::Reorder(ids)));
                self.ivars().events.send(event);
            }
            true
        }
    }
);

impl ProfileListTableDataSource {
    pub(super) fn set_editing(&self, editing: Option<(ProfileId, String)>) {
        self.ivars().editing.replace(editing);
    }
    fn row_view(&self, table: &NSTableView, row: NSInteger) -> Option<Retained<NSView>> {
        let row = usize::try_from(row).ok()?;
        let profile = self.ivars().profiles.borrow().get(row)?.clone();
        let height = 64.0;
        let editing_name = self
            .ivars()
            .editing
            .borrow()
            .as_ref()
            .filter(|(id, _)| id == profile.id())
            .map(|(_, name)| name.clone());
        let component = ProfileListItemComponent::build(
            ProfileListItemProps {
                saving: self.ivars().saving,
                profile,
                editing_name,
            },
            self.ivars().events.map(|event| ProfileListEvent::ProfileAction(Box::new(event))),
            (row, self.ivars().profiles.borrow().len()),
            objc2_foundation::NSRect::new(
                objc2_foundation::NSPoint::new(0.0, 0.0),
                objc2_foundation::NSSize::new(table.bounds().size.width, height),
            ),
            MainThreadMarker::new().expect("the table row callback should run on the AppKit thread because AppKit invokes it while updating the table"),
        );
        let view = component.view().retain();
        self.ivars().items.borrow_mut().push(component);
        Some(view)
    }

    fn pasteboard_writer(
        &self,
        row: NSInteger,
    ) -> Option<Retained<ProtocolObject<dyn NSPasteboardWriting>>> {
        if self.ivars().saving {
            return None;
        }
        let row = usize::try_from(row).ok()?;
        let id = self
            .ivars()
            .profiles
            .borrow()
            .get(row)?
            .id()
            .as_str()
            .to_owned();
        let item = NSPasteboardItem::new();
        item.setString_forType(&NSString::from_str(&id), &self.ivars().drag_type);
        Some(ProtocolObject::from_retained(item))
    }

    pub(super) fn new(
        mtm: MainThreadMarker,
        profiles: Vec<Profile>,
        saving: bool,
        editing: Option<(ProfileId, String)>,
        events: UiEventTarget<ProfileListEvent>,
        drag_type: Retained<NSString>,
    ) -> Retained<Self> {
        let allocated = mtm.alloc().set_ivars(ProfileListTableDataSourceIvars {
            profiles: RefCell::new(profiles),
            saving,
            editing: RefCell::new(editing),
            events,
            items: RefCell::new(Vec::new()),
            drag_type,
        });
        unsafe { msg_send![super(allocated), init] }
    }
}
