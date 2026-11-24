use super::DeviceKeyPasswordChangeInput;
use super::{AuthenticatorPromptPurpose, KeyOperationKind};
use crate::key_operation::AuthenticatorPromptKind;
use keyringd_core::service::DeviceKeyProtectionChange;
use keyringd_fido::AuthenticatorTransport;
use keyringd_fido_client::HybridTransportUnavailableReason;

pub enum KeyOperationCommand {
    CancelOperation,
    UpdateOperation {
        transport: AuthenticatorTransport,
    },
    PresentHybridQr {
        prompt_id: keyringd_fido_client::InteractionId,
        value: String,
        context: Option<AuthenticatorPromptPurpose>,
        security_key_available: bool,
        presented: tokio::sync::oneshot::Sender<()>,
        replace_active: bool,
        unavailable: Option<HybridTransportUnavailableReason>,
    },
    DismissHybridQr {
        prompt_id: keyringd_fido_client::InteractionId,
        acknowledged: tokio::sync::oneshot::Sender<()>,
    },
    UpdateHybridAvailability {
        reason: HybridTransportUnavailableReason,
    },
    AcknowledgeStaleHybridDismissal(tokio::sync::oneshot::Sender<()>),
    ShowAuthenticatorPrompt {
        operation: KeyOperationKind,
        transport: AuthenticatorTransport,
        prompt_kind: AuthenticatorPromptKind,
    },
    CloseOperationStatus,
    RequestPin {
        request_id: keyringd_fido_client::InteractionId,
        reason: String,
        attempts_left: Option<u32>,
        minimum_length: Option<u32>,
        new_pin: bool,
        response: tokio::sync::oneshot::Sender<Option<String>>,
    },
    AnswerPin {
        value: Option<String>,
        response: tokio::sync::oneshot::Sender<Option<String>>,
    },
    ActivateUsb(keyringd_fido_client::OperationId),
    RequestDeviceKeyPassword {
        profile_name: String,
        response: tokio::sync::oneshot::Sender<Option<String>>,
    },
    RequestNewDeviceKeyPassword {
        profile_name: String,
        response: tokio::sync::oneshot::Sender<Option<String>>,
    },
    RequestDeviceKeyPasswordChange {
        profile_name: String,
        change: DeviceKeyProtectionChange,
        response: tokio::sync::oneshot::Sender<Option<DeviceKeyPasswordChangeInput>>,
    },
    AnswerDeviceKeyPassword {
        value: Option<String>,
        response: tokio::sync::oneshot::Sender<Option<String>>,
    },
    AnswerDeviceKeyPasswordChange {
        value: Option<DeviceKeyPasswordChangeInput>,
        response: tokio::sync::oneshot::Sender<Option<DeviceKeyPasswordChangeInput>>,
    },
}

impl KeyOperationCommand {
    pub fn interaction_id(&self) -> Option<keyringd_fido_client::InteractionId> {
        match self {
            Self::PresentHybridQr { prompt_id, .. } | Self::DismissHybridQr { prompt_id, .. } => {
                Some(*prompt_id)
            }
            Self::RequestPin { request_id, .. } => Some(*request_id),
            _ => None,
        }
    }
}
