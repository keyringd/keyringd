use super::HybridPrompt;
use crate::key_operation::AuthenticatorPromptKind;
use crate::pin_prompt_window::SecurityKeyPinPrompt;
use keyringd_fido::AuthenticatorTransport;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProfileSetupInteractionScreen {
    Hidden,
    Preparing,
    Qr(HybridPrompt),
    Pin(SecurityKeyPinPrompt),
    Waiting {
        transport: Option<AuthenticatorTransport>,
        prompt_kind: Option<AuthenticatorPromptKind>,
        pin_requested: bool,
    },
    Cancelling,
}
