use crate::key_operation::AuthenticatorPromptPurpose;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HybridPrompt {
    pub operation_id: keyringd_fido_client::OperationId,
    pub value: String,
    pub context: AuthenticatorPromptPurpose,
    pub unavailable: Option<keyringd_fido_client::HybridTransportUnavailableReason>,
    pub security_key_available: bool,
}
