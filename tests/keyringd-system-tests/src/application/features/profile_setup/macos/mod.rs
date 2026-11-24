mod completion_page;
pub use completion_page::CompletionPage;
mod passkey_selection_page;
pub use passkey_selection_page::PasskeySelectionPage;
mod profile_form_page;
pub use profile_form_page::ProfileFormPage;
mod passkey_not_found_page;
pub use passkey_not_found_page::PasskeyNotFoundPage;
mod profile_setup_pin_view;
pub use profile_setup_pin_view::ProfileSetupPinView;
mod profile_setup_interaction_page;
pub use profile_setup_interaction_page::ProfileSetupInteractionPage;
mod profile_setup_qr_view;
pub use profile_setup_qr_view::ProfileSetupQrView;
mod macos_profile_setup_wizard;
pub(crate) use macos_profile_setup_wizard::HEADING;
pub use macos_profile_setup_wizard::MacosProfileSetupWizard;
mod progress_observation;
use progress_observation::observe_progress;

mod profile_setup_waiting_view;
pub use profile_setup_waiting_view::ProfileSetupWaitingView;

mod macos_profile_setup_window;
pub use macos_profile_setup_window::MacosProfileSetupWindow;
