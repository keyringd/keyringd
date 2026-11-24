use crate::key_operation::AuthenticatorPromptKind;
use keyringd_fido::AuthenticatorTransport;
use keyringd_fido_client::{HybridTransportUnavailableReason, InteractionId};

/// Presentation decisions shared by native authenticator prompt adapters.
#[derive(Default)]
pub(super) struct AuthenticatorPromptState {
    operation_active: bool,
    active_hybrid_id: Option<InteractionId>,
    hybrid_input_enabled: bool,
    hybrid_unavailable: Option<HybridTransportUnavailableReason>,
    prompt_requests: Vec<(AuthenticatorTransport, AuthenticatorPromptKind)>,
}

impl AuthenticatorPromptState {
    pub fn operation_active(&self) -> bool {
        self.operation_active
    }

    pub fn hybrid_visible(&self) -> bool {
        self.active_hybrid_id.is_some()
    }

    pub fn accepts_hybrid_input(&self, id: InteractionId) -> bool {
        self.active_hybrid_id == Some(id) && self.hybrid_input_enabled
    }

    pub fn hybrid_input_id(&self) -> Option<InteractionId> {
        self.active_hybrid_id.filter(|_| self.hybrid_input_enabled)
    }

    pub fn finish_hybrid_input(&mut self) {
        self.hybrid_input_enabled = false;
    }

    /// Transport candidates must not replace an active hybrid prompt.
    pub fn operation_started(&mut self) -> bool {
        if self.hybrid_visible() {
            return false;
        }
        self.operation_active = true;
        self.prompt_requests.clear();
        true
    }

    /// Returns whether the host must replace an existing hybrid view.
    pub fn hybrid_requested(&mut self, id: InteractionId) -> bool {
        self.operation_active = true;
        self.hybrid_input_enabled = true;
        self.prompt_requests.clear();
        self.active_hybrid_id.replace(id).is_some()
    }

    /// A stale dismissal must only be acknowledged, without closing the view.
    pub fn hybrid_dismissed(&mut self, id: InteractionId) -> bool {
        if self.active_hybrid_id != Some(id) {
            return false;
        }
        self.active_hybrid_id = None;
        true
    }

    /// Retains the reason for a QR prompt that may arrive later.
    pub fn hybrid_unavailable(&mut self, reason: HybridTransportUnavailableReason) -> bool {
        self.hybrid_unavailable = Some(reason);
        self.hybrid_visible()
    }

    pub fn unavailable_reason(&self) -> Option<HybridTransportUnavailableReason> {
        self.hybrid_unavailable
    }

    /// Authenticator prompts are independent per transport, but duplicate updates
    /// for the same transport and prompt kind do not recreate the view.
    pub fn record_prompt_request(
        &mut self,
        transport: &AuthenticatorTransport,
        prompt_kind: AuthenticatorPromptKind,
    ) -> bool {
        if self
            .prompt_requests
            .contains(&(transport.clone(), prompt_kind))
        {
            return false;
        }
        self.prompt_requests
            .retain(|(current, _)| current != transport);
        self.prompt_requests.push((transport.clone(), prompt_kind));
        true
    }

    pub fn pin_requested(&mut self) {
        self.prompt_requests.clear();
    }

    pub fn clear_prompts(&mut self) {
        *self = Self::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use keyringd_fido_client::OperationId;

    fn id(sequence: u64) -> InteractionId {
        InteractionId::new(OperationId::new(1), sequence)
    }

    keyringd_testkit::scenario! {
        group "AuthenticatorPromptState::hybrid_requested" {
            case "Stale dismissal does not close a newer prompt" {
                verify "acknowledge an old prompt ID without clearing its replacement" {
                    let mut state = AuthenticatorPromptState::default();
                    assert!(!state.hybrid_requested(id(1)));
                    assert!(state.hybrid_requested(id(2)));
                    assert!(!state.hybrid_dismissed(id(1)));
                    assert!(state.hybrid_visible());
                    assert!(!state.operation_started());
                    assert!(state.hybrid_dismissed(id(2)));
                    assert!(!state.hybrid_dismissed(id(2)));
                    assert!(state.operation_started());
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "AuthenticatorPromptState::hybrid_unavailable" {
            case "Availability is retained until the operation finishes" {
                verify "keep transport availability information until clearing the active prompts" {
                    let mut state = AuthenticatorPromptState::default();
                    let reason = HybridTransportUnavailableReason::NoBluetoothAdapter;
                    assert!(!state.hybrid_unavailable(reason));
                    state.hybrid_requested(id(1));
                    assert_eq!(state.unavailable_reason(), Some(reason));
                    assert!(state.hybrid_unavailable(reason));
                    state.clear_prompts();
                    assert!(!state.operation_active());
                    assert!(!state.hybrid_visible());
                    assert_eq!(state.unavailable_reason(), None);
                    assert!(!state.hybrid_dismissed(id(1)));
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "AuthenticatorPromptState::record_prompt_request" {
            case "Authenticator prompts are deduplicated per transport and kind" {
                verify "suppress repeat prompts only for the same transport and prompt kind" {
                    let mut state = AuthenticatorPromptState::default();
                    let usb = AuthenticatorTransport::usb();
                    let hybrid = AuthenticatorTransport::hybrid();
                    let presence = AuthenticatorPromptKind::UserPresence;
                    assert!(state.record_prompt_request(&usb, presence));
                    assert!(!state.record_prompt_request(&usb, presence));
                    assert!(state.record_prompt_request(&hybrid, presence));
                    assert!(state.record_prompt_request(&usb, AuthenticatorPromptKind::BuiltInUserVerification));
                    assert!(state.record_prompt_request(&usb, presence));
                    assert!(!state.record_prompt_request(&hybrid, presence));
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "AuthenticatorPromptState::record_prompt_request" {
            case "Pin and new stages allow presence to be shown again" {
                verify "allow another presence prompt after PIN input or a new processing stage" {
                    let mut state = AuthenticatorPromptState::default();
                    let usb = AuthenticatorTransport::usb();
                    let presence = AuthenticatorPromptKind::UserPresence;
                    assert!(state.record_prompt_request(&usb, presence));
                    state.pin_requested();
                    assert!(state.record_prompt_request(&usb, presence));
                    assert!(state.operation_started());
                    assert!(state.record_prompt_request(&usb, presence));
                    state.hybrid_requested(id(1));
                    assert!(state.record_prompt_request(&usb, presence));
                    state.clear_prompts();
                    assert!(state.record_prompt_request(&usb, presence));
                }
            }
        }
    }
}
