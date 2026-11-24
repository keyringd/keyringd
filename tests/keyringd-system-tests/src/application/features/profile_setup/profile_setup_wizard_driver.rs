use super::{
    CompletionPageDriver, PasskeyNotFoundPageDriver, PasskeySelectionPageDriver,
    ProfileFormPageDriver, ProfileSetupInteractionPageDriver, ProfileSetupProgress,
};
use anyhow::Result;

/// Navigation and observation of the entire wizard. Page actions belong to its pages.
pub trait ProfileSetupWizardDriver: Sized {
    type ProfileFormPage<'a>: ProfileFormPageDriver
    where
        Self: 'a;
    type PasskeySelectionPage<'a>: PasskeySelectionPageDriver
    where
        Self: 'a;
    type PasskeyNotFoundPage<'a>: PasskeyNotFoundPageDriver
    where
        Self: 'a;
    type InteractionPage<'a>: ProfileSetupInteractionPageDriver
    where
        Self: 'a;
    type CompletionPage<'a>: CompletionPageDriver
    where
        Self: 'a;
    fn back(&mut self) -> Result<()>;
    /// Observe without requiring a particular page, including during transitions.
    fn observe(&mut self) -> Result<ProfileSetupProgress>;
    fn profile_form_page(&mut self) -> Result<Self::ProfileFormPage<'_>>;
    fn passkey_selection_page(&mut self) -> Result<Self::PasskeySelectionPage<'_>>;
    fn passkey_not_found_page(&mut self) -> Result<Self::PasskeyNotFoundPage<'_>>;
    fn interaction_page(&mut self) -> Result<Self::InteractionPage<'_>>;
    fn completion_page(&mut self) -> Result<Self::CompletionPage<'_>>;
}
