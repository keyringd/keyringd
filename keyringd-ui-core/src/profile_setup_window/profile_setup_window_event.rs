use keyringd_core::application::PasskeyCredentialCandidate;
use keyringd_core::application::{ProfileSetupFailure, ProfileSetupProgress};
use keyringd_core::model::Profile;
use std::sync::Arc;

use super::ProfileSetupEvent;
use crate::key_operation::KeyOperationCommand;

/// User input and asynchronous results consumed by the setup window.
pub enum ProfileSetupWindowEvent {
    Input(ProfileSetupEvent),
    KeyOperation(KeyOperationCommand),
    Progressed(ProfileSetupProgress),
    CredentialSelectionRequested(Vec<PasskeyCredentialCandidate>),
    SecurityKeyActivationRejected,
    Finished(Arc<Result<Profile, ProfileSetupFailure>>),
    TrustEntriesOpened(Result<(), String>),
    PinRequested {
        request_id: keyringd_fido_client::InteractionId,
        prompt: crate::pin_prompt_window::SecurityKeyPinPrompt,
    },
}

impl From<ProfileSetupEvent> for ProfileSetupWindowEvent {
    fn from(event: ProfileSetupEvent) -> Self {
        Self::Input(event)
    }
}
