use keyringd_core::model::ProfileId;
use keyringd_core::model::passkey::CredentialId;

use keyringd_core::application::ProfileSetupRequest;

/// Commands emitted by the profile setup wizard reducer.
pub enum ProfileSetupWindowCommand {
    Start(ProfileSetupRequest),
    SelectCredential(Result<CredentialId, keyringd_core::application::ApplicationError>),
    CancelOperation,
    CloseWindow,
    OpenTrustEntries(ProfileId),
    UseUsb(keyringd_fido_client::OperationId),
    OperationSettled {
        close_window: bool,
    },
    RespondPin {
        request_id: keyringd_fido_client::InteractionId,
        command: crate::pin_prompt_window::PinPromptCommand,
    },
    Acknowledge(tokio::sync::oneshot::Sender<()>),
    KeyOperation(crate::key_operation::KeyOperationCommand),
}
