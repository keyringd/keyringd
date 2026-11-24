use super::KeyOperationEvent;
use keyringd_core::application::ActionId;
use std::collections::HashMap;

use std::sync::{Arc, Mutex};

use crate::runtime::UiEventSender;

use super::{Action, ActionRoute, ActionState, KeyOperationKind};

/// Routes UI interactions by action ID and serializes standalone prompts.
#[derive(Clone)]
pub struct ActionRegistry {
    states: Arc<Mutex<HashMap<ActionId, ActionState>>>,
    standalone_operation: Arc<tokio::sync::Mutex<()>>,
}

impl ActionRegistry {
    pub fn new() -> Self {
        Self {
            states: Arc::new(Mutex::new(HashMap::new())),
            standalone_operation: Arc::new(tokio::sync::Mutex::new(())),
        }
    }

    pub fn begin_owned(&self, operation: KeyOperationKind) -> Action {
        let id = ActionId::generate();
        let (events, receiver) = UiEventSender::channel();
        let cancellation = tokio_util::sync::CancellationToken::new();
        self.insert(
            id,
            operation,
            events.map(move |event| (id, event)),
            cancellation.clone(),
            None,
        );
        Action {
            id,
            events: Some(receiver),
            cancellation,
            registry: self.clone(),
        }
    }

    pub async fn begin_standalone(
        &self,
        events: UiEventSender<(ActionId, KeyOperationEvent)>,
    ) -> Action {
        let cancellation = tokio_util::sync::CancellationToken::new();
        let guard = Arc::clone(&self.standalone_operation).lock_owned().await;
        let id = ActionId::generate();
        self.insert(
            id,
            KeyOperationKind::AgentRequest,
            events.map(move |event| (id, event)),
            cancellation.clone(),
            Some(guard),
        );
        Action {
            id,
            events: None,
            cancellation,
            registry: self.clone(),
        }
    }

    fn insert(
        &self,
        id: ActionId,
        operation: KeyOperationKind,
        events: UiEventSender<KeyOperationEvent>,
        cancellation: tokio_util::sync::CancellationToken,
        standalone_guard: Option<tokio::sync::OwnedMutexGuard<()>>,
    ) {
        let span = id.span();
        span.record(
            "kind",
            match &operation {
                KeyOperationKind::ProfileSetup { .. } => "setup_profile",
                KeyOperationKind::DeviceKeyProtectionChange { .. } => {
                    "change_device_key_protection"
                }
                KeyOperationKind::DeviceKeyReplacement { .. } => "replace_device_key",
                KeyOperationKind::AgentRequest => "ssh_sign",
            },
        );
        self.states.lock().unwrap().insert(
            id,
            ActionState {
                span,
                operation,
                events,
                cancellation,
                _standalone_guard: standalone_guard,
            },
        );
    }

    /// The standalone-operation lock permits exactly one active agent flow.
    pub fn standalone_id(&self) -> Option<ActionId> {
        self.states
            .lock()
            .unwrap()
            .iter()
            .find(|(_, flow)| flow._standalone_guard.is_some())
            .map(|(id, _)| *id)
    }

    pub(super) fn finish(&self, id: ActionId) {
        let state = self.states.lock().unwrap().remove(&id);
        if let Some(state) = state {
            let _entered = state.span.enter();
            state.cancellation.cancel();
            tracing::debug!("action unregistered");
        }
    }

    pub fn cancel(&self, id: ActionId) {
        if let Some(cancellation) = self.cancellation(id) {
            cancellation.cancel();
        }
    }

    pub fn events(&self, id: ActionId) -> Option<UiEventSender<KeyOperationEvent>> {
        self.states
            .lock()
            .unwrap()
            .get(&id)
            .map(|flow| flow.events.clone())
    }

    pub fn span(&self, id: ActionId) -> Option<tracing::Span> {
        self.states
            .lock()
            .unwrap()
            .get(&id)
            .map(|state| state.span.clone())
    }

    pub fn cancellation(&self, id: ActionId) -> Option<tokio_util::sync::CancellationToken> {
        self.states
            .lock()
            .unwrap()
            .get(&id)
            .map(|flow| flow.cancellation.clone())
    }

    pub fn dispatch_context(&self, id: ActionId) -> Option<ActionRoute> {
        self.states
            .lock()
            .unwrap()
            .get(&id)
            .map(|flow| ActionRoute {
                events: flow.events.clone(),
                operation: flow.operation.clone(),
                cancellation: flow.cancellation.clone(),
            })
    }

    pub fn is_empty(&self) -> bool {
        self.states.lock().unwrap().is_empty()
    }

    pub fn contains(&self, id: ActionId) -> bool {
        self.states.lock().unwrap().contains_key(&id)
    }
}

impl Default for ActionRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::UiEventSender;
    use keyringd_core::application::PasskeyChoice;
    use std::time::Duration;

    keyringd_testkit::scenario! {
        group "ActionRegistry::cancel" {
            case "Owned actions keep routing and cancellation independent" {
                async: tokio;
                timeout: std::time::Duration::from_secs(60);
                verify "route each action's events by its own ID and cancel the actions independently" {
                    let registry = ActionRegistry::new();
                    let mut first = registry.begin_owned(KeyOperationKind::ProfileSetup {
                        passkey: PasskeyChoice::Existing,
                    });
                    let mut second = registry.begin_owned(KeyOperationKind::DeviceKeyReplacement {
                        profile_name: "Work".into(),
                    });
                    let first_id = first.id();
                    let mut first_events = first.take_events();
                    let first_cancellation = first.cancellation_token();
                    let second_id = second.id();
                    let mut second_events = second.take_events();
                    let second_cancellation = second.cancellation_token();
                    assert_ne!(first_id, second_id);
                    let context = registry.dispatch_context(first_id).unwrap();
                    assert!(matches!(
                        context.operation,
                        KeyOperationKind::ProfileSetup { .. }
                    ));
                    context
                        .events
                        .try_send(KeyOperationEvent::CancelRequested)
                        .unwrap();
                    registry
                        .events(second_id)
                        .unwrap()
                        .try_send(KeyOperationEvent::PromptDismissalRequested)
                        .unwrap();
                    assert!(matches!(first_events.receive().await,
                                Some((id, KeyOperationEvent::CancelRequested)) if id == first_id));
                    assert!(matches!(second_events.receive().await,
                                Some((id, KeyOperationEvent::PromptDismissalRequested)) if id == second_id));
                    first.cancel();
                    assert!(context.cancellation.is_cancelled());
                    assert!(first_cancellation.is_cancelled());
                    assert!(!second_cancellation.is_cancelled());
                    drop(first);
                    assert!(registry.dispatch_context(first_id).is_none());
                    assert!(registry.contains(second_id));
                    drop(second);
                    assert!(second_cancellation.is_cancelled());
                    assert!(registry.is_empty());
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "ActionRegistry::begin_standalone" {
            case "Standalone lock is held until removal and waiter cancellation does not leak it" {
                async: tokio;
                timeout: std::time::Duration::from_secs(60);
                verify "reserve the standalone slot through removal and release cancelled waiters" {
                    let registry = ActionRegistry::new();
                    let (sender, _events) = UiEventSender::channel();
                    let first = registry.begin_standalone(sender.clone()).await;
                    let first_id = first.id();
                    assert!(
                        tokio::time::timeout(
                            Duration::from_millis(10),
                            registry.begin_standalone(sender.clone())
                        )
                        .await
                        .is_err()
                    );
                    assert!(registry.contains(first_id));
                    drop(first);
                    let second = tokio::time::timeout(Duration::from_secs(1), registry.begin_standalone(sender))
                        .await
                        .unwrap();
                    let second_id = second.id();
                    assert_ne!(first_id, second_id);
                    assert!(matches!(
                        registry.dispatch_context(second_id).unwrap().operation,
                        KeyOperationKind::AgentRequest
                    ));
                    drop(second);
                    assert!(registry.is_empty());
                }
            }
        }
    }
}
