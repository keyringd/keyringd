mod profile_setup_wizard_view;
pub(in crate::app::ui) use profile_setup_wizard_view::ProfileSetupWizardView;
mod profile_form_page;
pub(in crate::app::ui) use profile_form_page::ProfileFormPage;
mod completion_page;
pub(in crate::app::ui) use completion_page::CompletionPage;
mod passkey_not_found_page;
pub(in crate::app::ui) use passkey_not_found_page::PasskeyNotFoundPage;
mod passkey_selection_page;
pub(in crate::app::ui) use passkey_selection_page::PasskeySelectionPage;
#[cfg(test)]
mod test_support;
