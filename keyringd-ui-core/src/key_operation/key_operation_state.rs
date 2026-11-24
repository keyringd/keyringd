use super::authenticator_prompt_state::AuthenticatorPromptState;
use super::{KeyOperationCommand, KeyOperationEvent};
use crate::ComponentModel;
#[derive(Default)]
pub struct KeyOperationState {
    prompts: AuthenticatorPromptState,
    pin_request: Option<keyringd_fido_client::InteractionId>,
}

impl KeyOperationState {
    pub fn operation_active(&self) -> bool {
        self.prompts.operation_active()
    }
    pub fn hybrid_visible(&self) -> bool {
        self.prompts.hybrid_visible()
    }
}

impl ComponentModel for KeyOperationState {
    type Event = KeyOperationEvent;
    type Command = KeyOperationCommand;

    fn update(&mut self, event: Self::Event) -> Option<Self::Command> {
        Some(match event {
            KeyOperationEvent::CancelRequested => {
                self.pin_request = None;
                self.prompts.clear_prompts();
                KeyOperationCommand::CancelOperation
            }
            KeyOperationEvent::HybridQrCancelled { prompt_id } => {
                if !self.prompts.accepts_hybrid_input(prompt_id) || self.pin_request.is_some() {
                    return None;
                }
                self.prompts.clear_prompts();
                KeyOperationCommand::CancelOperation
            }
            KeyOperationEvent::HybridQrUseSecurityKeyRequested { prompt_id } => {
                if !self.prompts.accepts_hybrid_input(prompt_id) || self.pin_request.is_some() {
                    return None;
                }
                self.prompts.finish_hybrid_input();
                KeyOperationCommand::ActivateUsb(prompt_id.operation_id())
            }
            KeyOperationEvent::OperationStarted { transport } => {
                if !self.prompts.operation_started() {
                    return None;
                }
                KeyOperationCommand::UpdateOperation { transport }
            }
            KeyOperationEvent::HybridQrRequested {
                prompt_id,
                value,
                context,
                security_key_available,
                presented,
            } => {
                let replace_active = self.prompts.hybrid_requested(prompt_id);
                self.pin_request = None;
                KeyOperationCommand::PresentHybridQr {
                    prompt_id,
                    value,
                    context,
                    security_key_available,
                    presented,
                    replace_active,
                    unavailable: self.prompts.unavailable_reason(),
                }
            }
            KeyOperationEvent::HybridQrDismissed {
                prompt_id,
                acknowledged,
            } if self.prompts.hybrid_dismissed(prompt_id) => KeyOperationCommand::DismissHybridQr {
                prompt_id,
                acknowledged,
            },
            KeyOperationEvent::HybridQrDismissed { acknowledged, .. } => {
                KeyOperationCommand::AcknowledgeStaleHybridDismissal(acknowledged)
            }
            KeyOperationEvent::HybridTransportUnavailable { reason } => {
                if !self.prompts.hybrid_unavailable(reason) {
                    return None;
                }
                KeyOperationCommand::UpdateHybridAvailability { reason }
            }
            KeyOperationEvent::AuthenticatorPromptRequested {
                operation,
                transport,
                prompt_kind,
            } => {
                if !self.prompts.record_prompt_request(&transport, prompt_kind) {
                    return None;
                }
                KeyOperationCommand::ShowAuthenticatorPrompt {
                    operation,
                    transport,
                    prompt_kind,
                }
            }
            KeyOperationEvent::PromptDismissalRequested => {
                self.pin_request = None;
                self.prompts.clear_prompts();
                KeyOperationCommand::CloseOperationStatus
            }
            KeyOperationEvent::PinRequested {
                request_id,
                reason,
                attempts_left,
                minimum_length,
                new_pin,
                response,
            } => {
                self.pin_request = Some(request_id);
                self.prompts.finish_hybrid_input();
                self.prompts.pin_requested();
                KeyOperationCommand::RequestPin {
                    request_id,
                    reason,
                    attempts_left,
                    minimum_length,
                    new_pin,
                    response,
                }
            }
            KeyOperationEvent::PinAnswered {
                request_id,
                value,
                response,
            } => {
                if self.pin_request != Some(request_id) {
                    let _ = response.send(None);
                    return None;
                }
                self.pin_request = None;
                KeyOperationCommand::AnswerPin { value, response }
            }
            KeyOperationEvent::UseSecurityKeyRequested => {
                let prompt_id = self.prompts.hybrid_input_id()?;
                if self.pin_request.is_some() {
                    return None;
                }
                self.prompts.finish_hybrid_input();
                KeyOperationCommand::ActivateUsb(prompt_id.operation_id())
            }
            KeyOperationEvent::DeviceKeyPasswordRequested {
                profile_name,
                response,
            } => KeyOperationCommand::RequestDeviceKeyPassword {
                profile_name,
                response,
            },
            KeyOperationEvent::NewDeviceKeyPasswordRequested {
                profile_name,
                response,
            } => KeyOperationCommand::RequestNewDeviceKeyPassword {
                profile_name,
                response,
            },
            KeyOperationEvent::DeviceKeyPasswordChangeRequested {
                profile_name,
                change,
                response,
            } => KeyOperationCommand::RequestDeviceKeyPasswordChange {
                profile_name,
                change,
                response,
            },
            KeyOperationEvent::DeviceKeyPasswordAnswered { value, response } => {
                KeyOperationCommand::AnswerDeviceKeyPassword { value, response }
            }
            KeyOperationEvent::DeviceKeyPasswordChangeAnswered { value, response } => {
                KeyOperationCommand::AnswerDeviceKeyPasswordChange { value, response }
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::key_operation::AuthenticatorPromptKind;
    use crate::key_operation::AuthenticatorPromptPurpose;
    use crate::key_operation::KeyOperationKind;
    use keyringd_fido::AuthenticatorTransport;
    use keyringd_fido_client::{InteractionId, OperationId};

    fn request_pin(
        state: &mut KeyOperationState,
        request_id: InteractionId,
    ) -> (
        tokio::sync::oneshot::Sender<Option<String>>,
        tokio::sync::oneshot::Receiver<Option<String>>,
    ) {
        let (response, receiver) = tokio::sync::oneshot::channel();
        let Some(KeyOperationCommand::RequestPin {
            request_id: delivered,
            response,
            ..
        }) = state.update(KeyOperationEvent::PinRequested {
            request_id,
            reason: "PIN".into(),
            attempts_left: None,
            minimum_length: None,
            new_pin: false,
            response,
        })
        else {
            panic!("PIN request must reach the native adapter");
        };
        assert_eq!(delivered, request_id);
        (response, receiver)
    }

    keyringd_testkit::scenario! {
        group "KeyOperationState::update" {
            case "Pin answers reject older requests operations and duplicate submission" {
                verify "reject PIN replies with a mismatched ID or a consumed request" {
                    let first = InteractionId::new(OperationId::new(1), 2);
                    for current_id in [
                        InteractionId::new(OperationId::new(1), 3),
                        InteractionId::new(OperationId::new(2), 2),
                    ] {
                        let mut state = KeyOperationState::default();
                        let (old, mut old_result) = request_pin(&mut state, first);
                        let (current, mut current_result) = request_pin(&mut state, current_id);
                        assert!(
                            state
                                .update(KeyOperationEvent::PinAnswered {
                                    request_id: first,
                                    value: Some("old".into()),
                                    response: old,
                                })
                                .is_none()
                        );
                        assert_eq!(old_result.try_recv().unwrap(), None);
                        let Some(KeyOperationCommand::AnswerPin { value, response }) =
                            state.update(KeyOperationEvent::PinAnswered {
                                request_id: current_id,
                                value: Some("current".into()),
                                response: current,
                            })
                        else {
                            panic!("current PIN answer");
                        };
                        response.send(value).unwrap();
                        assert_eq!(current_result.try_recv().unwrap(), Some("current".into()));
                        let (response, mut duplicate) = tokio::sync::oneshot::channel();
                        assert!(
                            state
                                .update(KeyOperationEvent::PinAnswered {
                                    request_id: current_id,
                                    value: Some("duplicate".into()),
                                    response
                                })
                                .is_none()
                        );
                        assert_eq!(duplicate.try_recv().unwrap(), None);
                    }
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "KeyOperationState::update" {
            case "Pin input is invalidated by cancellation or completion" {
                verify "invalidate the pending PIN when cancellation or prompt dismissal occurs" {
                    for terminal in [
                        KeyOperationEvent::CancelRequested,
                        KeyOperationEvent::PromptDismissalRequested,
                    ] {
                        let mut state = KeyOperationState::default();
                        let request_id = InteractionId::new(OperationId::new(1), 2);
                        let (response, mut receiver) = request_pin(&mut state, request_id);
                        state.update(terminal);
                        assert!(
                            state
                                .update(KeyOperationEvent::PinAnswered {
                                    request_id,
                                    value: Some("late".into()),
                                    response
                                })
                                .is_none()
                        );
                        assert_eq!(receiver.try_recv().unwrap(), None);
                    }
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "KeyOperationState::update" {
            case "Qr actions reject stale prompts and cancel after usb selection" {
                verify "require the current QR ID and ignore cancellation after accepting a USB switch" {
                    let mut state = KeyOperationState::default();
                    let first = InteractionId::new(OperationId::new(1), 1);
                    let current = InteractionId::new(OperationId::new(1), 2);
                    for prompt_id in [first, current] {
                        let (presented, _) = tokio::sync::oneshot::channel();
                        state.update(KeyOperationEvent::HybridQrRequested {
                            prompt_id,
                            value: "FIDO:/test".into(),
                            context: Some(AuthenticatorPromptPurpose::AgentRequest),
                            security_key_available: true,
                            presented,
                        });
                    }
                    assert!(
                        state
                            .update(KeyOperationEvent::HybridQrCancelled { prompt_id: first })
                            .is_none()
                    );
                    assert!(
                        state
                            .update(KeyOperationEvent::HybridQrUseSecurityKeyRequested { prompt_id: first })
                            .is_none()
                    );
                    assert!(matches!(
                        state.update(KeyOperationEvent::HybridQrUseSecurityKeyRequested { prompt_id: current }),
                        Some(KeyOperationCommand::ActivateUsb(operation)) if operation == current.operation_id()
                    ));
                    assert!(
                        state
                            .update(KeyOperationEvent::HybridQrCancelled { prompt_id: current })
                            .is_none()
                    );
                    assert!(
                        state
                            .update(KeyOperationEvent::HybridQrUseSecurityKeyRequested { prompt_id: current })
                            .is_none()
                    );
                    let (acknowledged, mut receiver) = tokio::sync::oneshot::channel();
                    let Some(KeyOperationCommand::DismissHybridQr { acknowledged, .. }) =
                        state.update(KeyOperationEvent::HybridQrDismissed {
                            prompt_id: current,
                            acknowledged,
                        })
                    else {
                        panic!("USB selection must still allow QR cleanup acknowledgement");
                    };
                    acknowledged.send(()).unwrap();
                    assert!(receiver.try_recv().is_ok());
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "KeyOperationState::update" {
            case "Qr close after switching to pin cannot cancel the operation" {
                verify "ignore a QR close event once the PIN prompt has replaced it" {
                    let mut state = KeyOperationState::default();
                    let prompt_id = InteractionId::new(OperationId::new(1), 1);
                    let (presented, _) = tokio::sync::oneshot::channel();
                    state.update(KeyOperationEvent::HybridQrRequested {
                        prompt_id,
                        value: "FIDO:/test".into(),
                        context: Some(AuthenticatorPromptPurpose::AgentRequest),
                        security_key_available: true,
                        presented,
                    });
                    let request_id = InteractionId::new(OperationId::new(1), 2);
                    let (response, _) = request_pin(&mut state, request_id);
                    assert!(
                        state
                            .update(KeyOperationEvent::HybridQrCancelled { prompt_id })
                            .is_none()
                    );
                    assert!(matches!(
                        state.update(KeyOperationEvent::PinAnswered {
                            request_id,
                            value: Some("current PIN".into()),
                            response,
                        }),
                        Some(KeyOperationCommand::AnswerPin { .. })
                    ));
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "KeyOperationState::update" {
            case "Device key password requests and answers preserve the response channel" {
                async: tokio;
                timeout: std::time::Duration::from_secs(60);
                verify "deliver the password request and reply on the original channel" {
                    for new_password in [false, true] {
                        for value in [None, Some("password".to_owned())] {
                            let mut state = KeyOperationState::default();
                            let (response, receiver) = tokio::sync::oneshot::channel();
                            let event = if new_password {
                                KeyOperationEvent::NewDeviceKeyPasswordRequested {
                                    profile_name: "Work".into(),
                                    response,
                                }
                            } else {
                                KeyOperationEvent::DeviceKeyPasswordRequested {
                                    profile_name: "Work".into(),
                                    response,
                                }
                            };
                            let (profile_name, response) = match state.update(event).unwrap() {
                                KeyOperationCommand::RequestNewDeviceKeyPassword {
                                    profile_name,
                                    response,
                                } if new_password => (profile_name, response),
                                KeyOperationCommand::RequestDeviceKeyPassword {
                                    profile_name,
                                    response,
                                } if !new_password => (profile_name, response),
                                _ => panic!("expected the matching password request"),
                            };
                            assert_eq!(profile_name, "Work");
                            let Some(KeyOperationCommand::AnswerDeviceKeyPassword {
                                value: answer,
                                response,
                            }) = state.update(KeyOperationEvent::DeviceKeyPasswordAnswered {
                                value: value.clone(),
                                response,
                            })
                            else {
                                panic!("expected a password answer");
                            };
                            response.send(answer).unwrap();
                            assert_eq!(receiver.await.unwrap(), value);
                        }
                    }
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "KeyOperationState::update" {
            case "Password change preserves the requested action and answer" {
                async: tokio;
                timeout: std::time::Duration::from_secs(60);
                verify "forward the password-change kind and its answer to the waiting backend" {
                    use super::super::DeviceKeyPasswordChangeInput;
                    use keyringd_core::service::DeviceKeyProtectionChange;
                    for action in [
                        DeviceKeyProtectionChange::Enable,
                        DeviceKeyProtectionChange::ChangePassword,
                        DeviceKeyProtectionChange::Disable,
                    ] {
                        let mut state = KeyOperationState::default();
                        let (response, receiver) = tokio::sync::oneshot::channel();
                        let Some(KeyOperationCommand::RequestDeviceKeyPasswordChange {
                            profile_name,
                            change,
                            response,
                        }) = state.update(KeyOperationEvent::DeviceKeyPasswordChangeRequested {
                            profile_name: "Work".into(),
                            change: action,
                            response,
                        })
                        else {
                            panic!("expected a password change request");
                        };
                        assert_eq!(profile_name, "Work");
                        assert_eq!(change, action);
                        let value = match action {
                            DeviceKeyProtectionChange::Enable => DeviceKeyPasswordChangeInput::Enable {
                                new_password: "new".into(),
                            },
                            DeviceKeyProtectionChange::ChangePassword => DeviceKeyPasswordChangeInput::Change {
                                current_password: "old".into(),
                                new_password: "new".into(),
                            },
                            DeviceKeyProtectionChange::Disable => DeviceKeyPasswordChangeInput::Disable {
                                current_password: "old".into(),
                            },
                        };
                        let Some(KeyOperationCommand::AnswerDeviceKeyPasswordChange { value, response }) = state
                            .update(KeyOperationEvent::DeviceKeyPasswordChangeAnswered {
                                value: Some(value),
                                response,
                            })
                        else {
                            panic!("expected a password change answer");
                        };
                        assert!(response.send(value).is_ok());
                        match (action, receiver.await.unwrap().unwrap()) {
                            (
                                DeviceKeyProtectionChange::Enable,
                                DeviceKeyPasswordChangeInput::Enable { new_password },
                            ) => assert_eq!(new_password, "new"),
                            (
                                DeviceKeyProtectionChange::ChangePassword,
                                DeviceKeyPasswordChangeInput::Change {
                                    current_password,
                                    new_password,
                                },
                            ) => {
                                assert_eq!(current_password, "old");
                                assert_eq!(new_password, "new");
                            }
                            (
                                DeviceKeyProtectionChange::Disable,
                                DeviceKeyPasswordChangeInput::Disable { current_password },
                            ) => assert_eq!(current_password, "old"),
                            _ => panic!("password change answer lost its action"),
                        }
                    }
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "KeyOperationState::update" {
            case "Cancellation and transport choice generate commands" {
                verify "dispatch the expected command for each cancellation and transport choice" {
                    let mut state = KeyOperationState::default();
                    assert!(matches!(
                        state.update(KeyOperationEvent::CancelRequested),
                        Some(KeyOperationCommand::CancelOperation)
                    ));
                    assert!(
                        state
                            .update(KeyOperationEvent::UseSecurityKeyRequested)
                            .is_none()
                    );
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "KeyOperationState::update" {
            case "View usb intent retains current operation and rejects duplicates" {
                verify "send one USB selection for the operation whose QR is visible" {
                    let mut state = KeyOperationState::default();
                    assert!(
                        state
                            .update(KeyOperationEvent::UseSecurityKeyRequested)
                            .is_none()
                    );
                    let operation = OperationId::new(7);
                    let (presented, _) = tokio::sync::oneshot::channel();
                    state.update(KeyOperationEvent::HybridQrRequested {
                        prompt_id: InteractionId::new(operation, 1),
                        value: "FIDO:/test".into(),
                        context: Some(AuthenticatorPromptPurpose::AgentRequest),
                        security_key_available: true,
                        presented,
                    });
                    assert!(
                        matches!(state.update(KeyOperationEvent::UseSecurityKeyRequested), Some(KeyOperationCommand::ActivateUsb(target)) if target == operation)
                    );
                    assert!(
                        state
                            .update(KeyOperationEvent::UseSecurityKeyRequested)
                            .is_none()
                    );
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "KeyOperationState::update" {
            case "Qr protocol retains payloads and does not dismiss a newer prompt" {
                verify "forward each QR payload and retain the newer prompt on a stale acknowledgement" {
                    let mut state = KeyOperationState::default();
                    let id = |sequence| InteractionId::new(OperationId::new(1), sequence);
                    for sequence in [1, 2] {
                        let (presented, _receiver) = tokio::sync::oneshot::channel();
                        let command = state.update(KeyOperationEvent::HybridQrRequested {
                            prompt_id: id(sequence),
                            value: "FIDO:/test".into(),
                            context: Some(AuthenticatorPromptPurpose::AgentRequest),
                            security_key_available: sequence == 1,
                            presented,
                        });
                        assert!(
                            matches!(command, Some(KeyOperationCommand::PresentHybridQr {
                                    value, replace_active, security_key_available, ..
                                }) if value == "FIDO:/test" && replace_active == (sequence == 2) && security_key_available == (sequence == 1))
                        );
                    }
                    assert!(
                        state
                            .update(KeyOperationEvent::OperationStarted {
                                transport: AuthenticatorTransport::usb()
                            })
                            .is_none()
                    );
                    let (acknowledged, _receiver) = tokio::sync::oneshot::channel();
                    assert!(matches!(
                        state.update(KeyOperationEvent::HybridQrDismissed {
                            prompt_id: id(1),
                            acknowledged
                        }),
                        Some(KeyOperationCommand::AcknowledgeStaleHybridDismissal(_))
                    ));
                    assert!(state.hybrid_visible());
                    assert!(matches!(
                        state.update(KeyOperationEvent::PromptDismissalRequested),
                        Some(KeyOperationCommand::CloseOperationStatus)
                    ));
                    assert!(!state.operation_active());
                    assert!(!state.hybrid_visible());
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "KeyOperationState::update" {
            case "Pin allows the same presence prompt to be shown again" {
                verify "allow a presence prompt to recur after processing a PIN request" {
                    let mut state = KeyOperationState::default();
                    let event = || KeyOperationEvent::AuthenticatorPromptRequested {
                        operation: KeyOperationKind::AgentRequest,
                        transport: AuthenticatorTransport::usb(),
                        prompt_kind: AuthenticatorPromptKind::UserPresence,
                    };
                    assert!(matches!(
                        state.update(event()),
                        Some(KeyOperationCommand::ShowAuthenticatorPrompt { .. })
                    ));
                    assert!(state.update(event()).is_none());
                    let (response, _receiver) = tokio::sync::oneshot::channel();
                    assert!(matches!(
                        state.update(KeyOperationEvent::PinRequested {
                            request_id: keyringd_fido_client::InteractionId::new(
                                keyringd_fido_client::OperationId::new(1),
                                2
                            ),
                            reason: "PIN".into(),
                            attempts_left: None,
                            minimum_length: None,
                            new_pin: false,
                            response,
                        }),
                        Some(KeyOperationCommand::RequestPin { .. })
                    ));
                    assert!(matches!(
                        state.update(event()),
                        Some(KeyOperationCommand::ShowAuthenticatorPrompt { .. })
                    ));
                }
            }
        }
    }
}
