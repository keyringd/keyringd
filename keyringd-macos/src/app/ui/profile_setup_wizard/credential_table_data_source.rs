use crate::app::ui::Layout;
use crate::app::ui::profile_setup_wizard::PasskeyCredentialSelection;
use crate::app::ui::profile_setup_wizard::ProfileSetupEvent;
use keyringd_core::application::PasskeyCredentialCandidate;
use keyringd_ui_core::runtime::UiEventTarget;
use objc2::rc::Retained;
use objc2::runtime::NSObjectProtocol;
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send};
use objc2_app_kit::{
    NSAccessibility, NSControlTextEditingDelegate, NSTableCellView, NSTableColumn, NSTableView,
    NSTableViewDataSource, NSTableViewDelegate, NSTextField, NSView,
};
use objc2_foundation::{NSIndexSet, NSInteger, NSNotification, NSObject, NSString};
use std::cell::{Cell, RefCell};

pub(super) struct CredentialTableDataSourceIvars {
    table: Retained<NSTableView>,
    candidates: RefCell<Vec<PasskeyCredentialCandidate>>,
    rendering: Cell<bool>,
    events: UiEventTarget<ProfileSetupEvent>,
}

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "KeyringdCredentialTableDataSource"]
    #[ivars = CredentialTableDataSourceIvars]
    pub(super) struct CredentialTableDataSource;

    unsafe impl NSObjectProtocol for CredentialTableDataSource {}
    unsafe impl NSControlTextEditingDelegate for CredentialTableDataSource {}

    unsafe impl NSTableViewDataSource for CredentialTableDataSource {
        #[unsafe(method(numberOfRowsInTableView:))]
        fn number_of_rows(&self, _table: &NSTableView) -> NSInteger {
            self.ivars().candidates.borrow().len() as NSInteger
        }
    }

    unsafe impl NSTableViewDelegate for CredentialTableDataSource {
        #[unsafe(method_id(tableView:viewForTableColumn:row:))]
        fn row_view(
            &self,
            _table: &NSTableView,
            _column: Option<&NSTableColumn>,
            row: NSInteger,
        ) -> Option<Retained<NSView>> {
            self.make_row_view(row)
        }

        #[unsafe(method(tableViewSelectionDidChange:))]
        fn selection_changed(&self, _notification: &NSNotification) {
            if !self.ivars().rendering.get()
                && let Ok(index) = usize::try_from(self.ivars().table.selectedRow())
            {
                self.ivars()
                    .events
                    .send(ProfileSetupEvent::CredentialChoiceChanged(index));
            }
        }
    }
);

impl CredentialTableDataSource {
    fn make_row_view(&self, row: NSInteger) -> Option<Retained<NSView>> {
        let index = usize::try_from(row).ok()?;
        let candidates = self.ivars().candidates.borrow();
        let candidate = candidates.get(index)?;
        let mtm = self.mtm();
        let cell = NSTableCellView::new(mtm);
        let label = NSTextField::labelWithString(&NSString::from_str(candidate.label()), mtm);
        label.setAccessibilityIdentifier(Some(&NSString::from_str(&format!(
            "profile-setup.credential.{index}"
        ))));
        cell.addSubview(&label);
        unsafe {
            cell.setTextField(Some(&label));
        }
        Layout::prepare(&label);
        label
            .leadingAnchor()
            .constraintEqualToAnchor_constant(&cell.leadingAnchor(), 8.0)
            .setActive(true);
        label
            .trailingAnchor()
            .constraintEqualToAnchor_constant(&cell.trailingAnchor(), -8.0)
            .setActive(true);
        label
            .centerYAnchor()
            .constraintEqualToAnchor(&cell.centerYAnchor())
            .setActive(true);
        Some(cell.into_super())
    }

    pub(super) fn new(
        table: Retained<NSTableView>,
        events: UiEventTarget<ProfileSetupEvent>,
        mtm: MainThreadMarker,
    ) -> Retained<Self> {
        let allocated = mtm.alloc().set_ivars(CredentialTableDataSourceIvars {
            table,
            candidates: RefCell::new(Vec::new()),
            rendering: Cell::new(false),
            events,
        });
        unsafe { msg_send![super(allocated), init] }
    }

    pub(super) fn render(&self, selection: &PasskeyCredentialSelection) {
        let changed = self.ivars().candidates.borrow().as_slice() != selection.candidates();
        self.ivars().rendering.set(true);
        if changed {
            self.ivars()
                .candidates
                .replace(selection.candidates().to_vec());
            self.ivars().table.reloadData();
        }
        if !selection.candidates().is_empty() {
            self.ivars().table.selectRowIndexes_byExtendingSelection(
                &NSIndexSet::indexSetWithIndex(selection.selected_index()),
                false,
            );
        }
        self.ivars().rendering.set(false);
        if changed && let Some(window) = self.ivars().table.window() {
            window.makeFirstResponder(Some(&self.ivars().table));
        }
    }
}
