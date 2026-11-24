use super::HybridPrompt;
use crate::key_operation::AuthenticatorPromptKind;
use keyringd_fido::AuthenticatorTransport;

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct TransportPrompts {
    pub hybrid: Option<HybridPrompt>,
    pub selected_transport: Option<AuthenticatorTransport>,
    pub usb_requested: bool,
    pub prompted_transports: Vec<AuthenticatorTransport>,
    pub prompt_kind: Option<AuthenticatorPromptKind>,
    pub pin_requested: bool,
}
