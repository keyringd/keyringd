mod profile_setup_action_target;
mod profile_setup_wizard_view;

pub(crate) use profile_setup_action_target::ProfileSetupActionTarget;
pub(crate) use profile_setup_wizard_view::ProfileSetupWizardView;

pub(crate) use keyringd_ui_core::profile_setup_window::{
    EditingPage, PasskeyCredentialSelection, PasskeyNotFoundChoice,
};

pub(crate) use keyringd_ui_core::profile_setup_window::{ProfileSetupEvent, ProfileSetupScreen};

mod profile_setup_controls;
use profile_setup_controls::ProfileSetupControls;

mod passkey_not_found_page;
use passkey_not_found_page::PasskeyNotFoundPage;

mod passkey_selection_page;
use passkey_selection_page::PasskeySelectionPage;

mod profile_form_view;
pub(crate) use profile_form_view::ProfileFormView;

mod profile_name_field_delegate;
use profile_name_field_delegate::ProfileNameFieldDelegate;

mod credential_table_data_source;
use credential_table_data_source::CredentialTableDataSource;
