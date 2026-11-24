mod profile_setup_wizard_driver;
pub use profile_setup_wizard_driver::ProfileSetupWizardDriver;
mod profile_setup_window_driver;
pub use profile_setup_window_driver::ProfileSetupWindowDriver;
mod profile_form_page_driver;
pub use profile_form_page_driver::ProfileFormPageDriver;
mod passkey_selection_page_driver;
pub use passkey_selection_page_driver::PasskeySelectionPageDriver;
mod passkey_not_found_page_driver;
pub use passkey_not_found_page_driver::PasskeyNotFoundPageDriver;
mod profile_setup_interaction_page_driver;
pub use profile_setup_interaction_page_driver::ProfileSetupInteractionPageDriver;
mod profile_setup_qr_view_driver;
pub use profile_setup_qr_view_driver::ProfileSetupQrViewDriver;
mod completion_page_driver;
pub use completion_page_driver::CompletionPageDriver;
mod profile_setup_step;
pub use profile_setup_step::ProfileSetupStep;
#[cfg(target_os = "linux")]
pub mod linux;
#[cfg(any(
    target_os = "macos",
    all(target_os = "linux", feature = "test-support")
))]
pub mod macos;
#[cfg(target_os = "windows")]
pub mod windows;

mod profile_setup_form;
pub use profile_setup_form::ProfileSetupForm;
mod profile_setup_completion;
pub use profile_setup_completion::ProfileSetupCompletion;
mod profile_setup_progress;
pub use profile_setup_progress::ProfileSetupProgress;

#[cfg(unix)]
mod profile_setup_pin_view_driver;
#[cfg(unix)]
pub use profile_setup_pin_view_driver::ProfileSetupPinViewDriver;
#[cfg(unix)]
mod profile_setup_waiting_view_driver;
#[cfg(unix)]
pub use profile_setup_waiting_view_driver::ProfileSetupWaitingViewDriver;

mod profile_setup_pin_content;
pub use profile_setup_pin_content::ProfileSetupPinContent;
