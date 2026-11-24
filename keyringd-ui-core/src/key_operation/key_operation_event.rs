use super::DeviceKeyPasswordChangeInput;
use super::{AuthenticatorPromptPurpose, KeyOperationKind};
use crate::key_operation::AuthenticatorPromptKind;
use keyringd_core::service::DeviceKeyProtectionChange;
use keyringd_fido::AuthenticatorTransport;
use keyringd_fido_client::{HybridTransportUnavailableReason, InteractionId};

/// Inputs delivered to the UI owning a key-operation flow.
pub enum KeyOperationEvent {
    CancelRequested,
    HybridQrCancelled {
        prompt_id: InteractionId,
    },
    HybridQrUseSecurityKeyRequested {
        prompt_id: InteractionId,
    },
    OperationStarted {
        transport: AuthenticatorTransport,
    },
    HybridQrRequested {
        prompt_id: InteractionId,
        value: String,
        context: Option<AuthenticatorPromptPurpose>,
        security_key_available: bool,
        presented: tokio::sync::oneshot::Sender<()>,
    },
    HybridQrDismissed {
        prompt_id: InteractionId,
        acknowledged: tokio::sync::oneshot::Sender<()>,
    },
    HybridTransportUnavailable {
        reason: HybridTransportUnavailableReason,
    },
    AuthenticatorPromptRequested {
        operation: KeyOperationKind,
        transport: AuthenticatorTransport,
        prompt_kind: AuthenticatorPromptKind,
    },
    PromptDismissalRequested,
    PinRequested {
        request_id: keyringd_fido_client::InteractionId,
        reason: String,
        attempts_left: Option<u32>,
        minimum_length: Option<u32>,
        new_pin: bool,
        response: tokio::sync::oneshot::Sender<Option<String>>,
    },
    PinAnswered {
        request_id: keyringd_fido_client::InteractionId,
        value: Option<String>,
        response: tokio::sync::oneshot::Sender<Option<String>>,
    },
    UseSecurityKeyRequested,
    DeviceKeyPasswordRequested {
        profile_name: String,
        response: tokio::sync::oneshot::Sender<Option<String>>,
    },
    NewDeviceKeyPasswordRequested {
        profile_name: String,
        response: tokio::sync::oneshot::Sender<Option<String>>,
    },
    DeviceKeyPasswordChangeRequested {
        profile_name: String,
        change: DeviceKeyProtectionChange,
        response: tokio::sync::oneshot::Sender<Option<DeviceKeyPasswordChangeInput>>,
    },
    DeviceKeyPasswordAnswered {
        value: Option<String>,
        response: tokio::sync::oneshot::Sender<Option<String>>,
    },
    DeviceKeyPasswordChangeAnswered {
        value: Option<DeviceKeyPasswordChangeInput>,
        response: tokio::sync::oneshot::Sender<Option<DeviceKeyPasswordChangeInput>>,
    },
}

impl KeyOperationEvent {
    pub fn interaction_id(&self) -> Option<InteractionId> {
        match self {
            Self::PinRequested { request_id, .. } | Self::PinAnswered { request_id, .. } => {
                Some(*request_id)
            }
            Self::HybridQrRequested { prompt_id, .. }
            | Self::HybridQrDismissed { prompt_id, .. }
            | Self::HybridQrCancelled { prompt_id }
            | Self::HybridQrUseSecurityKeyRequested { prompt_id } => Some(*prompt_id),
            _ => None,
        }
    }
    /// Releases pending requests when their UI owner no longer accepts the flow.
    pub fn reject(self) {
        match self {
            Self::HybridQrDismissed { acknowledged, .. } => {
                let _ = acknowledged.send(());
            }
            Self::PinRequested { response, .. }
            | Self::PinAnswered { response, .. }
            | Self::DeviceKeyPasswordRequested { response, .. }
            | Self::NewDeviceKeyPasswordRequested { response, .. }
            | Self::DeviceKeyPasswordAnswered { response, .. } => {
                let _ = response.send(None);
            }
            Self::DeviceKeyPasswordChangeRequested { response, .. }
            | Self::DeviceKeyPasswordChangeAnswered { response, .. } => {
                let _ = response.send(None);
            }
            // An undisplayed QR must not receive a successful presentation acknowledgement.
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use keyringd_fido_client::OperationId;

    keyringd_testkit::scenario! {
        group "KeyOperationEvent::reject" {
            case "Rejecting a stale prompt releases requests without submitting old input" {
                async: tokio;
                timeout: std::time::Duration::from_secs(60);
                verify "release the stale prompt's response channels without forwarding input" {
                    for variant in 0..5 {
                        let (response, receiver) = tokio::sync::oneshot::channel();
                        let event = match variant {
                            0 => KeyOperationEvent::PinRequested {
                                request_id: keyringd_fido_client::InteractionId::new(
                                    keyringd_fido_client::OperationId::new(1),
                                    2,
                                ),
                                reason: "test".into(),
                                attempts_left: None,
                                minimum_length: None,
                                new_pin: false,
                                response,
                            },
                            1 => KeyOperationEvent::PinAnswered {
                                request_id: keyringd_fido_client::InteractionId::new(
                                    keyringd_fido_client::OperationId::new(1),
                                    2,
                                ),
                                value: Some("old PIN".into()),
                                response,
                            },
                            2 => KeyOperationEvent::DeviceKeyPasswordRequested {
                                profile_name: "Work".into(),
                                response,
                            },
                            3 => KeyOperationEvent::NewDeviceKeyPasswordRequested {
                                profile_name: "Work".into(),
                                response,
                            },
                            _ => KeyOperationEvent::DeviceKeyPasswordAnswered {
                                value: Some("old password".into()),
                                response,
                            },
                        };
                        event.reject();
                        assert_eq!(receiver.await.unwrap(), None);
                    }
                    for answered in [false, true] {
                        let (response, receiver) = tokio::sync::oneshot::channel();
                        let event = if answered {
                            KeyOperationEvent::DeviceKeyPasswordChangeAnswered {
                                value: Some(DeviceKeyPasswordChangeInput::Enable {
                                    new_password: "old input".into(),
                                }),
                                response,
                            }
                        } else {
                            KeyOperationEvent::DeviceKeyPasswordChangeRequested {
                                profile_name: "Work".into(),
                                change: DeviceKeyProtectionChange::Enable,
                                response,
                            }
                        };
                        event.reject();
                        assert!(receiver.await.unwrap().is_none());
                    }
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "KeyOperationEvent::reject" {
            case "Rejecting qr events acknowledges dismissal but never claims presentation" {
                async: tokio;
                timeout: std::time::Duration::from_secs(60);
                verify "release a pending QR dismissal acknowledgement and reject QR presentation" {
                    let prompt_id = InteractionId::new(OperationId::new(1), 1);
                    let (acknowledged, dismissal) = tokio::sync::oneshot::channel();
                    KeyOperationEvent::HybridQrDismissed {
                        prompt_id,
                        acknowledged,
                    }
                    .reject();
                    assert!(dismissal.await.is_ok());
                    let (presented, presentation) = tokio::sync::oneshot::channel();
                    KeyOperationEvent::HybridQrRequested {
                        prompt_id,
                        value: "FIDO:/test".into(),
                        context: Some(AuthenticatorPromptPurpose::AgentRequest),
                        security_key_available: false,
                        presented,
                    }
                    .reject();
                    assert!(presentation.await.is_err());
                }
            }
        }
    }
}
