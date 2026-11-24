use keyringd_core::application::ActionId;
use keyringd_fido_client::InteractionId;

use std::collections::HashMap;
use std::sync::Mutex;

use keyringd_fido_client::OperationId;
use tokio::sync::watch;

/// Routes FIDO interactions and retains their state until native dismissal completes.
pub struct FidoOperationRegistry {
    operations: Mutex<HashMap<OperationId, OperationState>>,
}

struct OperationState {
    span: tracing::Span,
    action_id: ActionId,
    prompt: Option<InteractionId>,
    usb_activation: Option<watch::Sender<bool>>,
}

impl Default for FidoOperationRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl FidoOperationRegistry {
    pub fn new() -> Self {
        Self {
            operations: Mutex::new(HashMap::new()),
        }
    }

    pub fn register(&self, operation_id: OperationId, action_id: ActionId) {
        self.operations.lock().unwrap().insert(
            operation_id,
            OperationState {
                span: tracing::Span::current(),
                action_id,
                prompt: None,
                usb_activation: None,
            },
        );
    }

    #[cfg(feature = "test-support")]
    pub(super) fn action_ids(&self) -> Vec<ActionId> {
        self.operations
            .lock()
            .unwrap()
            .values()
            .map(|state| state.action_id)
            .collect()
    }

    pub fn active_operation(&self, flow_id: ActionId) -> Option<OperationId> {
        self.operations
            .lock()
            .unwrap()
            .iter()
            .find(|(_, state)| state.action_id == flow_id)
            .map(|(id, _)| *id)
    }

    pub fn action_id(&self, operation_id: OperationId) -> Option<ActionId> {
        self.operations
            .lock()
            .unwrap()
            .get(&operation_id)
            .map(|state| state.action_id)
    }

    pub fn span(&self, operation_id: OperationId) -> Option<tracing::Span> {
        self.operations
            .lock()
            .unwrap()
            .get(&operation_id)
            .map(|state| state.span.clone())
    }

    pub fn set_usb_activation(&self, operation_id: OperationId, activation: watch::Sender<bool>) {
        if let Some(state) = self.operations.lock().unwrap().get_mut(&operation_id) {
            state.usb_activation = Some(activation);
        }
    }

    pub fn activate_usb(&self, flow_id: ActionId, operation_id: OperationId) -> bool {
        let activation = self
            .operations
            .lock()
            .unwrap()
            .get(&operation_id)
            .filter(|state| state.action_id == flow_id)
            .and_then(|state| state.usb_activation.clone());
        if let Some(activation) = activation {
            activation.send_replace(true);
            true
        } else {
            false
        }
    }

    pub fn set_prompt(&self, operation_id: OperationId, prompt: InteractionId) {
        if let Some(state) = self.operations.lock().unwrap().get_mut(&operation_id) {
            state.prompt = Some(prompt);
        }
    }

    /// Take the QR to dismiss. The action and USB route remain registered until
    /// the caller receives the native dismissal acknowledgement and removes them.
    pub fn begin_dismissal(&self, operation_id: OperationId) -> Option<InteractionId> {
        let mut operations = self.operations.lock().unwrap();
        let state = operations.get_mut(&operation_id)?;
        state.prompt.take()
    }

    pub fn remove(&self, operation_id: OperationId) {
        self.operations.lock().unwrap().remove(&operation_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use keyringd_fido_client::InteractionId;

    keyringd_testkit::scenario! {
        group "FidoOperationRegistry::register" {
            case "Routing and usb activation are isolated by action" {
                async: tokio;
                timeout: std::time::Duration::from_secs(60);
                verify "send notifications and USB activation only to the owning action" {
                    let registry = FidoOperationRegistry::new();
                    let first = OperationId::new(1);
                    let second = OperationId::new(2);
                    let first_action_id = ActionId::new(10);
                    let second_action_id = ActionId::new(20);
                    registry.register(first, first_action_id);
                    registry.register(second, second_action_id);
                    let (first_activation, first_value) = watch::channel(false);
                    let (second_activation, second_value) = watch::channel(false);
                    registry.set_usb_activation(first, first_activation);
                    registry.set_usb_activation(second, second_activation);
                    assert!(matches!(registry.action_id(first), Some(id) if id == first_action_id));
                    assert!(registry.activate_usb(second_action_id, second));
                    assert!(!*first_value.borrow());
                    assert!(*second_value.borrow());
                    assert!(!registry.activate_usb(ActionId::new(99), second));
                    registry.remove(second);
                    assert!(registry.action_id(second).is_none());
                    assert!(!registry.activate_usb(second_action_id, second));
                    assert!(registry.action_id(first).is_some());
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "FidoOperationRegistry::register" {
            case "Delayed usb activation cannot select a later operation in the same action" {
                async: tokio;
                timeout: std::time::Duration::from_secs(60);
                verify "ignore USB activation carrying an earlier operation ID on the same action" {
                    let registry = FidoOperationRegistry::new();
                    let action_id = ActionId::new(10);
                    let previous = OperationId::new(1);
                    let current = OperationId::new(2);
                    let (previous_activation, previous_value) = watch::channel(false);
                    let (current_activation, current_value) = watch::channel(false);
                    for operation in [previous, current] {
                        registry.register(operation, action_id);
                    }
                    registry.set_usb_activation(previous, previous_activation);
                    registry.set_usb_activation(current, current_activation);
                    assert!(registry.activate_usb(action_id, previous));
                    assert!(*previous_value.borrow());
                    assert!(!*current_value.borrow());
                    registry.remove(previous);
                    assert!(!registry.activate_usb(action_id, previous));
                    assert!(!*current_value.borrow());
                    assert!(!registry.activate_usb(ActionId::new(99), current));
                    assert!(!*current_value.borrow());
                    assert!(registry.activate_usb(action_id, current));
                    assert!(*current_value.borrow());
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "FidoOperationRegistry::begin_dismissal" {
            case "Dismissal retains routing until acknowledgement" {
                async: tokio;
                timeout: std::time::Duration::from_secs(60);
                verify "keep the route registered until native QR closure is acknowledged" {
                    let registry = FidoOperationRegistry::new();
                    let operation = OperationId::new(1);
                    let action_id = ActionId::new(1);
                    registry.register(operation, action_id);
                    let interaction = InteractionId::new(operation, 1);
                    registry.set_prompt(operation, interaction);
                    registry.set_usb_activation(operation, watch::channel(false).0);
                    assert_eq!(
                        registry.begin_dismissal(operation).unwrap(),
                        interaction
                    );
                    assert!(registry.begin_dismissal(operation).is_none());
                    assert!(registry.action_id(operation).is_some());
                    assert!(registry.activate_usb(action_id, operation));
                    registry.remove(operation);
                    assert!(registry.action_id(operation).is_none());
                    registry.set_prompt(operation, interaction);
                    assert!(registry.begin_dismissal(operation).is_none());
                }
            }
        }
    }
}
