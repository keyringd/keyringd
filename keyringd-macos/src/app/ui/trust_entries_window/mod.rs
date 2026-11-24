#[allow(clippy::module_inception)]
mod trust_entries_window;
mod trust_entries_window_action_target;
mod trust_entries_window_command_handler;
mod trust_entry_view;

pub(crate) use keyringd_ui_core::trust_entries_window::TrustEntryViewProps;
pub(crate) use keyringd_ui_core::trust_entries_window::{TrustEntriesEvent, TrustEntriesState};
pub(crate) use trust_entries_window::TrustEntriesWindow;
use trust_entries_window_action_target::TrustEntriesWindowActionTarget;
use trust_entries_window_command_handler::TrustEntriesWindowCommandHandler;
use trust_entry_view::TrustEntryView;
