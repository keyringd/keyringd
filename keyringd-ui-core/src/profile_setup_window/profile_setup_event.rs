use super::PasskeyNotFoundChoice;
use keyringd_core::application::PasskeyChoice;
use keyringd_ssh_agent_protocol::SshSignatureAlgorithm;

/// Repeatable input from the setup window controls.
#[derive(Clone)]
pub enum ProfileSetupEvent {
    NameChanged(String),
    SignatureAlgorithmChanged(SshSignatureAlgorithm),
    AdvancedSettingsToggled,
    PasskeyChanged(PasskeyChoice),
    SetupRequested,
    BackToSetup,
    BackRequested,
    PasskeyNotFoundChoiceChanged(PasskeyNotFoundChoice),
    PasskeyNotFoundChoiceConfirmed,
    CredentialChoiceChanged(usize),
    CredentialSelectionConfirmed,
    PinCompleted {
        request_id: keyringd_fido_client::InteractionId,
        command: crate::pin_prompt_window::PinPromptCommand,
    },
    UseSecurityKeyRequested(keyringd_fido_client::OperationId),
    ShowTrustEntries,
    Dismissed,
}
