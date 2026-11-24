use crate::text;
use keyringd_core::application::ActionId;

use crate::runtime::{UiEventSendError, UiEventSender};

use super::KeyOperationEvent;
use crate::key_operation::Action;

use crate::key_operation::ActionRegistry;
use crate::key_operation::KeyOperationKind;
use crate::runtime::UiEventReceiver;
use keyringd_core::application::PasskeyChoice;
use keyringd_fido_client::OperationId;

/// Coordinates action interactions, UI event delivery, and cancellation.
pub struct KeyOperationCoordinator {
    standalone_events: UiEventSender<(ActionId, KeyOperationEvent)>,
    pub actions: ActionRegistry,
    pub fido_operations: crate::key_operation::FidoOperationRegistry,
}

impl KeyOperationCoordinator {
    /// Restores log ancestry for native UI callbacks and queued events.
    pub fn log_span(
        &self,
        id: ActionId,
        operation_id: Option<OperationId>,
        interaction_id: Option<keyringd_fido_client::InteractionId>,
    ) -> tracing::Span {
        let execution = self.actions.span(id).unwrap_or_else(|| id.span());
        let operation_id = interaction_id
            .map(|id| id.operation_id())
            .or(operation_id)
            .or_else(|| self.fido_operations.active_operation(id));
        let Some(operation_id) = operation_id else {
            return execution;
        };
        let operation = self
            .fido_operations
            .span(operation_id)
            .unwrap_or_else(|| execution.in_scope(|| operation_id.span()));
        interaction_id.map_or_else(|| operation.clone(), |id| operation.in_scope(|| id.span()))
    }
    /// Registers SSH-request interaction routing before any key or FIDO service runs.
    pub async fn run_agent_action<F: std::future::Future>(&self, future: F) -> F::Output {
        let action = self
            .actions
            .begin_standalone(self.standalone_events.clone())
            .await;
        let id = action.id();
        action
            .run(async {
                let result = future.await;
                while let Some(operation) = self.fido_operations.active_operation(id) {
                    self.dismiss_fido_ui(operation).await;
                }
                result
            })
            .await
    }
    pub fn channel() -> (
        std::sync::Arc<Self>,
        UiEventReceiver<(ActionId, KeyOperationEvent)>,
    ) {
        let (sender, receiver) = UiEventSender::channel();
        (
            std::sync::Arc::new(Self {
                standalone_events: sender,
                actions: ActionRegistry::new(),
                fido_operations: crate::key_operation::FidoOperationRegistry::new(),
            }),
            receiver,
        )
    }

    pub fn begin_profile_setup(&self, passkey: PasskeyChoice) -> Action {
        self.begin_action(KeyOperationKind::ProfileSetup { passkey })
    }

    pub fn begin_device_key_protection_change(&self, profile_name: String) -> Action {
        self.begin_action(KeyOperationKind::DeviceKeyProtectionChange { profile_name })
    }

    pub fn begin_device_key_replacement(&self, profile_name: String) -> Action {
        self.begin_action(KeyOperationKind::DeviceKeyReplacement { profile_name })
    }

    fn begin_action(&self, operation: KeyOperationKind) -> Action {
        self.actions.begin_owned(operation)
    }

    pub fn cancel_action(&self, id: ActionId) {
        self.actions.cancel(id);
    }

    /// Called after the backend settles and the caller is ready to close its UI.
    /// Keep the route registered until native QR dismissal is acknowledged.
    pub(super) async fn dismiss_fido_ui(&self, id: OperationId) {
        use tracing::Instrument;
        let span = self.fido_operations.span(id).unwrap_or_else(|| id.span());
        async {
            let Some(action_id) = self.fido_operations.action_id(id) else {
                return;
            };
            if let Some(prompt) = self.fido_operations.begin_dismissal(id) {
                self.dismiss_hybrid_qr_code(action_id, prompt).await;
            }
            self.request_prompt_dismissal(action_id);
            self.fido_operations.remove(id);
        }
        .instrument(span)
        .await
    }

    #[cfg(feature = "test-support")]
    pub fn active_operations(&self) -> usize {
        self.fido_operations.action_ids().len()
    }

    #[cfg(feature = "test-support")]
    pub fn cancel_all_operations(&self) {
        for action_id in self.fido_operations.action_ids() {
            self.actions.cancel(action_id);
        }
    }

    pub fn accepts_input(&self, owner: Option<ActionId>, input: ActionId) -> bool {
        owner.or_else(|| self.actions.standalone_id()) == Some(input)
            && self
                .actions
                .cancellation(input)
                .is_some_and(|token| !token.is_cancelled())
    }

    pub fn event_sender(&self, id: ActionId) -> Option<UiEventSender<KeyOperationEvent>> {
        self.actions.events(id)
    }
}

use crate::key_operation::AuthenticatorPromptKind;
use keyringd_fido::ctap::Pin;
use keyringd_fido_client::{
    FidoInteractionHandler, FidoInteractionHandlerError, InteractionId, InteractionResponse,
    NewPinInteraction, OperationStatus, PinInteraction,
};

impl FidoInteractionHandler for KeyOperationCoordinator {
    fn request_transport(
        &self,
        request: keyringd_fido_client::TransportInteraction,
    ) -> futures_util::future::BoxFuture<
        '_,
        Result<
            keyringd_fido_client::InteractionResponse<
                Option<keyringd_fido::AuthenticatorTransport>,
            >,
            keyringd_fido_client::FidoInteractionHandlerError,
        >,
    > {
        Box::pin(async move {
            Ok(keyringd_fido_client::InteractionResponse::new(
                request.id(),
                Some(keyringd_fido::AuthenticatorTransport::hybrid()),
            ))
        })
    }

    fn on_operation_status_changed(
        &self,
        event: OperationStatus,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<(), FidoInteractionHandlerError>> + Send + '_>,
    > {
        Box::pin(async move {
            let operation_id = event.operation_id();
            if matches!(event, OperationStatus::BackendFinished { .. }) {
                // A completed backend may leave the unavailable QR visible for
                // a USB choice. Only dismiss_fido_ui closes the interaction route.
                tracing::debug!(op_id = operation_id.get(), "FIDO backend finished");
                return Ok(());
            }
            let app_operation = super::fido_operation_context::CURRENT_FIDO_OPERATION
                .try_with(std::sync::Arc::clone)
                .ok();
            if matches!(event, OperationStatus::OperationStarted { .. }) {
                let action_id = ActionId::current().ok_or_else(handler_inactive_error)?;
                self.fido_operations.register(operation_id, action_id);
                if let Some(flow) = &app_operation {
                    *flow.active.lock().expect("the FIDO operation mutex should be unpoisoned because its critical sections should not panic") = Some(operation_id);
                }
                return Ok(());
            }
            let action_id = self
                .fido_operations
                .action_id(operation_id)
                .ok_or_else(handler_inactive_error)?;
            let session_id = action_id;
            match event {
                OperationStatus::OperationStarted { .. } => unreachable!(),
                OperationStatus::Processing { .. } => {}
                OperationStatus::HybridQrReady { qr_uri, .. } => {
                    let activation = app_operation
                        .as_ref()
                        .map(|flow| flow.usb.clone())
                        .unwrap_or_else(|| tokio::sync::watch::channel(false).0);
                    self.show_hybrid_qr_code(
                        session_id,
                        InteractionId::new(operation_id, 1),
                        &qr_uri,
                        activation,
                        true,
                    )
                    .await?;
                }
                OperationStatus::AuthenticatorSelectionRequired { transport, .. } => {
                    self.request_authenticator_prompt(
                        session_id,
                        transport,
                        AuthenticatorPromptKind::AuthenticatorSelection,
                    );
                }
                OperationStatus::UserPresenceRequired { transport, .. } => {
                    self.request_authenticator_prompt(
                        session_id,
                        transport,
                        AuthenticatorPromptKind::UserPresence,
                    );
                }
                OperationStatus::BuiltInUserVerification { transport, .. } => {
                    self.request_authenticator_prompt(
                        session_id,
                        transport,
                        AuthenticatorPromptKind::BuiltInUserVerification,
                    );
                }
                OperationStatus::TransportSelected { transport, .. } => {
                    self.event_sender(session_id)
                        .ok_or_else(handler_inactive_error)?
                        .try_send(KeyOperationEvent::OperationStarted { transport })
                        .map_err(handler_channel_error)?;
                }
                OperationStatus::HybridTransportUnavailable { reason, .. } => {
                    self.event_sender(session_id)
                        .ok_or_else(handler_inactive_error)?
                        .try_send(KeyOperationEvent::HybridTransportUnavailable { reason })
                        .map_err(handler_channel_error)?;
                }
                OperationStatus::BackendFinished { .. } => unreachable!(),
            }
            Ok(())
        })
    }

    fn request_pin(
        &self,
        request: PinInteraction,
    ) -> futures_util::future::BoxFuture<
        '_,
        Result<InteractionResponse<Option<Pin>>, FidoInteractionHandlerError>,
    > {
        Box::pin(async move {
            let action_id = self
                .fido_operations
                .action_id(request.id().operation_id())
                .ok_or_else(handler_inactive_error)?;
            let reason = text::PIN_PROMPT_PIN_REASON;
            let pin = self
                .request_session_pin(
                    action_id,
                    request.id(),
                    reason,
                    request
                        .attempts_remaining()
                        .and_then(|v| u32::try_from(v).ok()),
                    None,
                    false,
                )
                .await?
                .map(|value| Pin::from_text(&value))
                .transpose()
                .map_err(FidoInteractionHandlerError::new)?;
            Ok(InteractionResponse::new(request.id(), pin))
        })
    }

    fn request_new_pin(
        &self,
        request: NewPinInteraction,
    ) -> futures_util::future::BoxFuture<
        '_,
        Result<InteractionResponse<Option<Pin>>, FidoInteractionHandlerError>,
    > {
        Box::pin(async move {
            let action_id = self
                .fido_operations
                .action_id(request.id().operation_id())
                .ok_or_else(handler_inactive_error)?;
            let minimum_length = request.minimum_length();
            let reason = crate::pin_prompt_window::new_pin_reason(
                minimum_length,
                request.rejected_by_authenticator(),
            );
            let pin = self
                .request_session_pin(
                    action_id,
                    request.id(),
                    &reason,
                    None,
                    u32::try_from(minimum_length).ok(),
                    true,
                )
                .await?
                .map(|value| Pin::from_text_with_minimum(&value, minimum_length))
                .transpose()
                .map_err(FidoInteractionHandlerError::new)?;
            Ok(InteractionResponse::new(request.id(), pin))
        })
    }
}

fn handler_channel_error(_error: UiEventSendError) -> FidoInteractionHandlerError {
    FidoInteractionHandlerError::new(std::io::Error::new(
        std::io::ErrorKind::BrokenPipe,
        "key operation UI event channel closed",
    ))
}

fn handler_inactive_error() -> FidoInteractionHandlerError {
    FidoInteractionHandlerError::new(std::io::Error::new(
        std::io::ErrorKind::NotConnected,
        "key operation UI session is inactive",
    ))
}

fn handler_cancelled_error() -> FidoInteractionHandlerError {
    FidoInteractionHandlerError::new(std::io::Error::new(
        std::io::ErrorKind::Interrupted,
        "key operation UI was cancelled",
    ))
}

impl KeyOperationCoordinator {
    #[cfg(feature = "test-support")]
    pub async fn present_test_hybrid_qr(
        &self,
        operation_id: OperationId,
        uri: &str,
    ) -> Result<(), FidoInteractionHandlerError> {
        let action_id = self
            .fido_operations
            .action_id(operation_id)
            .ok_or_else(handler_inactive_error)?;
        let activation = super::fido_operation_context::CURRENT_FIDO_OPERATION
            .try_with(|flow| flow.usb.clone())
            .unwrap_or_else(|_| tokio::sync::watch::channel(false).0);
        self.show_hybrid_qr_code(
            action_id,
            InteractionId::new(operation_id, 1),
            uri,
            activation,
            true,
        )
        .await?;
        Ok(())
    }

    pub async fn wait_for_session_cancellation(&self, session_id: ActionId) {
        let cancellation = self.actions.cancellation(session_id);
        let Some(cancellation) = cancellation else {
            return;
        };
        cancellation.cancelled().await;
    }

    pub async fn show_hybrid_qr_code(
        &self,
        session_id: ActionId,
        interaction_id: InteractionId,
        qr_code: &str,
        usb_activation: tokio::sync::watch::Sender<bool>,
        security_key_available: bool,
    ) -> Result<(), FidoInteractionHandlerError> {
        interaction_id
            .scope(async {
                let operation = self.actions.dispatch_context(session_id)
                    .ok_or_else(handler_inactive_error)?.operation;
                let prompt_id = interaction_id;
                self.fido_operations
                    .set_usb_activation(interaction_id.operation_id(), usb_activation);
                self.fido_operations
                    .set_prompt(interaction_id.operation_id(), prompt_id);
                let context = operation.authenticator_prompt_purpose();
                let (presented, receiver) = tokio::sync::oneshot::channel();
                self.event_sender(session_id)
                    .ok_or_else(handler_inactive_error)?
                    .try_send(KeyOperationEvent::HybridQrRequested {
                        prompt_id,
                        value: qr_code.to_owned(),
                        context,
                        security_key_available,
                        presented,
                    })
                    .map_err(handler_channel_error)?;
                tokio::select! {
                    biased;
                    _ = self.wait_for_session_cancellation(session_id) => return Err(handler_cancelled_error()),
                    result = receiver => result.map_err(FidoInteractionHandlerError::new)?,
                }
                Ok(())
            })
            .await
    }

    pub async fn dismiss_hybrid_qr_code(&self, session_id: ActionId, prompt: InteractionId) {
        prompt
            .scope(async {
                let (acknowledged, receiver) = tokio::sync::oneshot::channel();
                let Some(sender) = self.event_sender(session_id) else {
                    tracing::debug!("QR dismissal skipped because the UI flow has ended");
                    return;
                };
                match sender.try_send(KeyOperationEvent::HybridQrDismissed {
                    prompt_id: prompt,
                    acknowledged,
                }) {
                    Ok(()) => {
                        if let Err(error) = receiver.await {
                            tracing::warn!(?error, "QR dismissal was not acknowledged by the UI");
                        }
                    }
                    Err(error) => {
                        tracing::warn!( %error, "could not send QR dismissal to the UI")
                    }
                }
            })
            .await
    }

    fn request_authenticator_prompt(
        &self,
        session_id: ActionId,
        transport: keyringd_fido::AuthenticatorTransport,
        prompt_kind: AuthenticatorPromptKind,
    ) {
        let Some(context) = self.actions.dispatch_context(session_id) else {
            return;
        };
        let _ = context
            .events
            .try_send(KeyOperationEvent::AuthenticatorPromptRequested {
                operation: context.operation,
                transport,
                prompt_kind,
            });
    }

    fn request_prompt_dismissal(&self, action_id: ActionId) {
        if let Some(sender) = self.event_sender(action_id) {
            let _ = sender.try_send(KeyOperationEvent::PromptDismissalRequested);
        }
    }

    pub async fn request_session_pin(
        &self,
        session_id: ActionId,
        request_id: InteractionId,
        reason: &str,
        attempts_left: Option<u32>,
        minimum_length: Option<u32>,
        new_pin: bool,
    ) -> Result<Option<String>, FidoInteractionHandlerError> {
        let (response, receiver) = tokio::sync::oneshot::channel();
        self.event_sender(session_id)
            .ok_or_else(handler_inactive_error)?
            .try_send(KeyOperationEvent::PinRequested {
                request_id,
                reason: reason.to_owned(),
                attempts_left,
                minimum_length,
                new_pin,
                response,
            })
            .map_err(handler_channel_error)?;
        receiver.await.map_err(FidoInteractionHandlerError::new)
    }
}

#[cfg(feature = "test-support")]
impl keyringd_core::test_support::TestFidoInteractionHandler for KeyOperationCoordinator {
    fn present_hybrid_qr<'a>(
        &'a self,
        operation_id: OperationId,
        uri: &'a str,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<(), FidoInteractionHandlerError>> + Send + 'a>,
    > {
        Box::pin(async move { self.present_test_hybrid_qr(operation_id, uri).await })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use keyringd_core::application::PasskeyChoice;

    keyringd_testkit::scenario! {
        group "KeyOperationCoordinator::run_agent_action" {
            case "Agent action keeps one id and route across fido operations" {
                async: tokio;
                timeout: std::time::Duration::from_secs(60);
                verify "reuse the SSH action ID and delivery route for consecutive FIDO attempts" {
                    let (coordinator, mut events) = KeyOperationCoordinator::channel();
                    let id = coordinator
                        .run_agent_action(keyringd_core::test_support::run_test_execution(async {
                            let id = ActionId::current().unwrap();
                            for operation_id in [OperationId::new(1), OperationId::new(2)] {
                                coordinator
                                    .on_operation_status_changed(OperationStatus::OperationStarted { operation_id })
                                    .await
                                    .unwrap();
                                assert_eq!(
                                    coordinator.fido_operations.action_id(operation_id),
                                    Some(id)
                                );
                                coordinator
                                    .on_operation_status_changed(OperationStatus::BackendFinished { operation_id })
                                    .await
                                    .unwrap();
                                assert_eq!(coordinator.fido_operations.action_id(operation_id), Some(id));
                                assert!(futures_util::poll!(std::pin::pin!(events.receive())).is_pending());
                                coordinator.dismiss_fido_ui(operation_id).await;
                                // FIDO completion must not end the enclosing SSH action's route.
                                assert!(coordinator.actions.contains(id));
                                assert!(matches!(events.receive().await,
                                    Some((source, KeyOperationEvent::PromptDismissalRequested)) if source == id));
                            }
                            id
                        }))
                        .await;
                    assert!(!coordinator.actions.contains(id));
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "KeyOperationCoordinator::run_agent_action" {
            case "Dropping agent action releases its route and standalone slot" {
                async: tokio;
                timeout: std::time::Duration::from_secs(60);
                verify "remove the dropped action route and release its standalone reservation" {
                    let (coordinator, _events) = KeyOperationCoordinator::channel();
                    let (started, receiver) = tokio::sync::oneshot::channel();
                    let task_coordinator = coordinator.clone();
                    let task = tokio::spawn(async move {
                        task_coordinator
                            .run_agent_action(keyringd_core::test_support::run_test_execution(async {
                                started.send(ActionId::current().unwrap()).unwrap();
                                std::future::pending::<()>().await;
                            }))
                            .await;
                    });
                    let previous = receiver.await.unwrap();
                    let cancellation = coordinator.actions.cancellation(previous).unwrap();
                    task.abort();
                    let _ = task.await;
                    assert!(cancellation.is_cancelled());
                    assert!(!coordinator.actions.contains(previous));
                    let next = coordinator
                        .run_agent_action(keyringd_core::test_support::run_test_execution(async {
                            ActionId::current().unwrap()
                        }))
                        .await;
                    assert_ne!(next, previous);
                }
            }
        }
    }
    keyringd_testkit::scenario! {
        group "KeyOperationCoordinator::dismiss_fido_ui" {
            case "Attempt completion waits for qr dismissal and preserves the action" {
                async: tokio;
                timeout: std::time::Duration::from_secs(60);
                verify "delay the UI dismissal event until QR acknowledgement and keep the action route active" {
                    for cancel_before_ack in [false, true] {
                        let (handle, _standalone_events) = KeyOperationCoordinator::channel();
                        let mut owner = handle.begin_profile_setup(PasskeyChoice::Existing);
                        let action_id = owner.id();
                        let mut events = owner.take_events();
                        let _cancellation = owner.cancellation_token();
                        let context = action_id
                            .scope(keyringd_core::test_support::run_test_execution(async {
                                super::super::FidoOperationContext::new(&handle)
                            }))
                            .await;
                        let action = context.execution_id.action_id();
                        let operation_id = OperationId::new(99);
                        let task_handle = handle.clone();
                        let task_context = context.clone();
                        let task = tokio::spawn(async move {
                            task_context
                                .scope(async {
                                    task_handle
                                        .on_operation_status_changed(OperationStatus::OperationStarted {
                                            operation_id,
                                        })
                                        .await
                                        .unwrap();
                                    let result = task_handle
                                        .on_operation_status_changed(OperationStatus::HybridQrReady {
                                            operation_id,
                                            qr_uri: "FIDO:/test".into(),
                                        })
                                        .await;
                                    if cancel_before_ack {
                                        assert!(result.is_err());
                                    } else {
                                        result.unwrap();
                                    }
                                    task_handle
                                        .on_operation_status_changed(OperationStatus::BackendFinished {
                                            operation_id,
                                        })
                                        .await
                                        .unwrap();
                                })
                                .await;
                            // UI dismissal must also work while the attempt context is current.
                            task_context.scope(task_context.dismiss_attempt_ui()).await;
                        });
                        let (_, KeyOperationEvent::HybridQrRequested { prompt_id: shown_id, presented, .. }) =
                            events.receive().await.unwrap()
                        else {
                            panic!("expected presentation");
                        };
                        assert!(!task.is_finished());
                        if cancel_before_ack {
                            context.cancellation.cancel();
                            drop(presented);
                        } else {
                            presented.send(()).unwrap();
                        }
                        let (_, KeyOperationEvent::HybridQrDismissed { prompt_id: dismissed_id, acknowledged }) =
                            events.receive().await.unwrap()
                        else {
                            panic!("dismissal must precede completion, even before presentation was acknowledged");
                        };
                        assert_eq!(dismissed_id, shown_id);
                        assert!(handle.fido_operations.action_id(operation_id).is_some());
                        assert!(!task.is_finished());
                        acknowledged.send(()).unwrap();
                        assert!(matches!(
                            events.receive().await,
                            Some((_, KeyOperationEvent::PromptDismissalRequested))
                        ));
                        task.await.unwrap();
                        assert!(handle.fido_operations.action_id(operation_id).is_none());
                        assert!(handle.actions.contains(action));
                        drop(context);
                        assert!(handle.actions.contains(action));
                        drop(owner);
                    }
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "KeyOperationCoordinator::event_sender" {
            case "Queued notifications keep their source after the next action starts" {
                async: tokio;
                timeout: std::time::Duration::from_secs(60);
                verify "deliver queued events with the original action ID after a new action starts" {
                    let (handle, mut events) = KeyOperationCoordinator::channel();
                    let first_action = handle
                        .actions
                        .begin_standalone(handle.standalone_events.clone())
                        .await;
                    let first = first_action.id();
                    handle
                        .event_sender(first)
                        .unwrap()
                        .send(KeyOperationEvent::PromptDismissalRequested);
                    drop(first_action);
                    let second_action = handle
                        .actions
                        .begin_standalone(handle.standalone_events.clone())
                        .await;
                    let second = second_action.id();
                    handle
                        .event_sender(second)
                        .unwrap()
                        .send(KeyOperationEvent::OperationStarted {
                            transport: keyringd_fido::AuthenticatorTransport::usb(),
                        });
                    assert!(
                        matches!(events.receive().await, Some((id, KeyOperationEvent::PromptDismissalRequested)) if id == first)
                    );
                    assert!(
                        matches!(events.receive().await, Some((id, KeyOperationEvent::OperationStarted { .. })) if id == second)
                    );
                    drop(second_action);
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "KeyOperationCoordinator::cancel_action" {
            case "Expired ui commands do not target a later standalone action" {
                async: tokio;
                timeout: std::time::Duration::from_secs(60);
                verify "ignore commands tied to a previous standalone action" {
                    let (handle, _events) = KeyOperationCoordinator::channel();
                    let first_action = handle
                        .actions
                        .begin_standalone(handle.standalone_events.clone())
                        .await;
                    let first = first_action.id();
                    drop(first_action);
                    let second_action = handle
                        .actions
                        .begin_standalone(handle.standalone_events.clone())
                        .await;
                    let second = second_action.id();
                    let cancellation = handle.actions.cancellation(second).unwrap();
                    assert!(!handle.accepts_input(None, first));
                    assert!(handle.accepts_input(None, second));
                    handle.cancel_action(first);
                    assert!(handle.accepts_input(None, second));
                    assert!(!cancellation.is_cancelled());
                    drop(second_action);
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "KeyOperationCoordinator::cancel_action" {
            case "Cancelling an owned action blocks new input without releasing its cleanup lifetime" {
                verify "reject new input for a cancelled action while keeping it registered for cleanup" {
                    let (coordinator, _) = KeyOperationCoordinator::channel();
                    let first = coordinator.begin_device_key_replacement("Work".into());
                    let second = coordinator.begin_device_key_replacement("Other".into());
                    assert!(coordinator.accepts_input(Some(first.id()), first.id()));
                    assert!(!coordinator.accepts_input(Some(first.id()), second.id()));
                    coordinator.cancel_action(first.id());
                    assert!(!coordinator.accepts_input(Some(first.id()), first.id()));
                    assert!(coordinator.actions.contains(first.id()));
                    assert!(coordinator.accepts_input(Some(second.id()), second.id()));
                    let first_id = first.id();
                    drop(first);
                    assert!(!coordinator.actions.contains(first_id));
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "KeyOperationCoordinator::request_session_pin" {
            case "Deliver parallel prompts by Action ID" {
                async: tokio;
                timeout: std::time::Duration::from_secs(10);
                given "Prepare KeyOperationCoordinator, PassKeyChoice, and KeyOperationEvent" {
                    let (interaction, _standalone_events) = KeyOperationCoordinator::channel();
                    let mut first = interaction.begin_profile_setup(PasskeyChoice::Existing);
                    let mut second = interaction.begin_device_key_protection_change("test".to_owned());
                    let first_id = first.id();
                    let second_id = second.id();
                    let mut first_events = first.take_events();
                    let _first_cancellation = first.cancellation_token();
                    let mut second_events = second.take_events();
                    let _second_cancellation = second.cancellation_token();
                }
                when "Request a PIN PROMPT in parallel from two Actions" {
                    let first_interaction = std::sync::Arc::clone(&interaction);
                    let first_prompt = tokio::spawn(async move {
                        first_interaction
                            .request_session_pin(
                                first_id,
                                keyringd_fido_client::InteractionId::new(
                                    keyringd_fido_client::OperationId::new(1),
                                    1,
                                ),
                                "first",
                                None,
                                None,
                                false,
                            )
                            .await
                    });
                    let second_interaction = std::sync::Arc::clone(&interaction);
                    let second_prompt = tokio::spawn(async move {
                        second_interaction
                            .request_session_pin(
                                second_id,
                                keyringd_fido_client::InteractionId::new(
                                    keyringd_fido_client::OperationId::new(2),
                                    1,
                                ),
                                "second",
                                None,
                                None,
                                false,
                            )
                            .await
                    });
                    let Some(KeyOperationEvent::PinRequested {
                        reason, response, ..
                    }) = first_events.receive().await.map(|(_, event)| event)
                    else {
                        panic!("first action did not receive its PIN prompt");
                    };
                }
                then "A PIN request for each Action arrives" {
                    assert_eq!(reason, "first");
                }
                when "Return a PIN to the requested action" {
                    response.send(Some("1111".to_owned())).unwrap();
                    let Some(KeyOperationEvent::PinRequested {
                        reason, response, ..
                    }) = second_events.receive().await.map(|(_, event)| event)
                    else {
                        panic!("second action did not receive its PIN prompt");
                    };
                }
                then "A PIN request for each Action arrives" {
                    assert_eq!(reason, "second");
                }
                when "Return a PIN to the requested action" {
                    response.send(Some("2222".to_owned())).unwrap();
                }
                then "Deliver parallel prompts by Action ID" {
                    assert_eq!(
                        first_prompt.await.unwrap().unwrap().as_deref(),
                        Some("1111")
                    );
                    assert_eq!(
                        second_prompt.await.unwrap().unwrap().as_deref(),
                        Some("2222")
                    );
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "KeyOperationCoordinator::cancel_action" {
            case "Cancellation wakes only the targeted action" {
                async: tokio;
                timeout: std::time::Duration::from_secs(10);
                given "Prepare KeyOperationCoordinator, PassKeyChoice, InteractionID" {
                    let (interaction, _standalone_events) = KeyOperationCoordinator::channel();
                    let first = interaction.begin_profile_setup(PasskeyChoice::Existing);
                    let second = interaction.begin_device_key_protection_change("test".to_owned());
                }
                when "Cancel the second while waiting for the cancellation of the first action" {
                    let first_interaction = std::sync::Arc::clone(&interaction);
                    let first_id = first.id();
                    let first_wait = tokio::spawn(async move {
                        first_interaction
                            .wait_for_session_cancellation(first_id)
                            .await;
                    });
                    interaction.cancel_action(second.id());
                    tokio::task::yield_now().await;
                }
                then "Canceling another action does not terminate the waiting action" {
                    assert!(!first_wait.is_finished());
                }
                when "Cancel a waiting action" {
                    interaction.cancel_action(first.id());
                }
                then "Cancel wait for the corresponding first action" {
                    tokio::time::timeout(std::time::Duration::from_secs(1), first_wait)
                                    .await
                                    .expect("cancelling the matching operation should notify its registered waiter before the timeout")
                                    .unwrap();
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "KeyOperationCoordinator::show_hybrid_qr_code" {
            case "Distinguish the Hybrid Prompt of the same FIDO Operation with an Interaction ID" {
                async: tokio;
                timeout: std::time::Duration::from_secs(10);
                given "Prepare KeyOperationCoordinator, PassKeyChoice, InteractionID" {
                    let (interaction, _standalone_events) = KeyOperationCoordinator::channel();
                    let mut action = interaction.begin_profile_setup(PasskeyChoice::Existing);
                    let action_id = action.id();
                    let mut requests = action.take_events();
                    let _cancellation = action.cancellation_token();
                }
                when "Request two Hybrid QR Prompts in order with the same action" {
                    let interaction_for_first = std::sync::Arc::clone(&interaction);
                    let first = tokio::spawn(async move {
                        interaction_for_first
                            .show_hybrid_qr_code(
                                action_id,
                                InteractionId::new(OperationId::new(1), 1),
                                "FIDO:/first",
                                tokio::sync::watch::channel(false).0,
                                true,
                            )
                            .await
                            .unwrap()
                    });
                    let KeyOperationEvent::HybridQrRequested {
                        prompt_id: first_id,
                        presented,
                        ..
                    } = requests.receive().await.unwrap().1
                    else {
                        panic!("first QR prompt was not received");
                    };
                    presented.send(()).unwrap();
                    first.await.unwrap();
                    let interaction_for_second = std::sync::Arc::clone(&interaction);
                    let second = tokio::spawn(async move {
                        interaction_for_second
                            .show_hybrid_qr_code(
                                action_id,
                                InteractionId::new(OperationId::new(1), 2),
                                "FIDO:/second",
                                tokio::sync::watch::channel(false).0,
                                true,
                            )
                            .await
                            .unwrap()
                    });
                    let KeyOperationEvent::HybridQrRequested {
                        prompt_id: second_id,
                        presented,
                        ..
                    } = requests.receive().await.unwrap().1
                    else {
                        panic!("second QR prompt was not received");
                    };
                    presented.send(()).unwrap();
                    second.await.unwrap();
                }
                then "Distinguish the Hybrid Prompt of the same FIDO Operation with an Interaction ID" {
                    assert_eq!(first_id.operation_id(), second_id.operation_id());
                    assert!(first_id.sequence() < second_id.sequence());
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "KeyOperationCoordinator::show_hybrid_qr_code" {
            case "AuthenticatorPromptPurpose progresses between FIDO Operations" {
                async: tokio;
                timeout: std::time::Duration::from_secs(10);
                given "Start Profile Setup Session with Existing Passkey" {
                    let (interaction, _standalone_events) = KeyOperationCoordinator::channel();
                    let mut action = interaction.begin_profile_setup(PasskeyChoice::Existing);
                    let action_id = action.id();
                    let mut requests = action.take_events();
                    let _cancellation = action.cancellation_token();
                }
                when "Show QR Prompt with 3 consecutive FIDO Operations" {
                    let mut contexts = Vec::new();
                    for operation in [1, 2, 3] {
                        let interaction_for_show = std::sync::Arc::clone(&interaction);
                        let show = tokio::spawn(async move {
                            interaction_for_show
                                .show_hybrid_qr_code(
                                    action_id,
                                    InteractionId::new(OperationId::new(operation), 1),
                                    "FIDO:/test",
                                    tokio::sync::watch::channel(false).0,
                                    true,
                                )
                                .await
                                .unwrap()
                        });
                        let KeyOperationEvent::HybridQrRequested {
                            context, presented, ..
                        } = requests.receive().await.unwrap().1
                        else {
                            panic!("QR prompt was not received");
                        };
                        contexts.push(context);
                        presented.send(()).unwrap();
                        show.await.unwrap();
                    }
                }
                then "Profile setup owns the purpose independently of QR request count" {
                    assert_eq!(
                        contexts,
                        [None, None, None]
                    );
                }
            }
        }
    }

    mod fido_interaction_handler {
        use super::*;

        use keyringd_core::application::PasskeyChoice;
        use keyringd_fido_client::OperationId;

        keyringd_testkit::scenario! {
            group "KeyOperationCoordinator::request_session_pin" {
                case "Pin input and user cancellation remain successful responses" {
                    async: tokio;
                    timeout: std::time::Duration::from_secs(60);
                    verify "return the PIN value or an empty successful cancellation response" {
                        for new_pin in [false, true] {
                            for answer in [None, Some("1234".to_owned())] {
                                let (handle, _standalone) = KeyOperationCoordinator::channel();
                                let mut action = handle.begin_profile_setup(PasskeyChoice::Existing);
                                let id = action.id();
                                let mut events = action.take_events();
                                let _cancellation = action.cancellation_token();
                                let task = tokio::spawn(async move {
                                    handle
                                        .request_session_pin(
                                            id,
                                            InteractionId::new(OperationId::new(1), 1),
                                            "PIN",
                                            None,
                                            None,
                                            new_pin,
                                        )
                                        .await
                                });
                                let KeyOperationEvent::PinRequested { response, .. } =
                                    events.receive().await.unwrap().1
                                else {
                                    panic!("expected a PIN request");
                                };
                                response.send(answer.clone()).unwrap();
                                assert_eq!(task.await.unwrap().unwrap(), answer);
                            }
                        }
                    }
                }
            }
        }

        keyringd_testkit::scenario! {
            group "KeyOperationCoordinator::request_session_pin" {
                case "Disconnected pin request channel is an error" {
                    async: tokio;
                    timeout: std::time::Duration::from_secs(60);
                    verify "return a handler error if the PIN request cannot be delivered" {
                        let (handle, _standalone) = KeyOperationCoordinator::channel();
                        let mut action = handle.begin_profile_setup(PasskeyChoice::Existing);
                        let id = action.id();
                        let events = action.take_events();
                        let _cancellation = action.cancellation_token();
                        drop(events);
                        assert!(
                            handle
                                .request_session_pin(
                                    id,
                                    InteractionId::new(OperationId::new(1), 1),
                                    "PIN",
                                    None,
                                    None,
                                    false
                                )
                                .await
                                .is_err()
                        );
                    }
                }
            }
        }

        keyringd_testkit::scenario! {
            group "KeyOperationCoordinator::request_session_pin" {
                case "Dropped pin response is an error instead of cancellation" {
                    async: tokio;
                    timeout: std::time::Duration::from_secs(60);
                    verify "return a handler error if the response sender disappears without replying" {
                        let (handle, _standalone) = KeyOperationCoordinator::channel();
                        let mut action = handle.begin_profile_setup(PasskeyChoice::Existing);
                        let id = action.id();
                        let mut events = action.take_events();
                        let _cancellation = action.cancellation_token();
                        let task = tokio::spawn(async move {
                            handle
                                .request_session_pin(
                                    id,
                                    InteractionId::new(OperationId::new(1), 1),
                                    "PIN",
                                    None,
                                    None,
                                    false,
                                )
                                .await
                        });
                        let KeyOperationEvent::PinRequested { response, .. } = events.receive().await.unwrap().1 else {
                            panic!("expected a PIN request");
                        };
                        drop(response);
                        assert!(task.await.unwrap().is_err());
                    }
                }
            }
        }

        keyringd_testkit::scenario! {
            group "KeyOperationCoordinator::show_hybrid_qr_code" {
                case "Dropped hybrid presentation acknowledgement is an error" {
                    async: tokio;
                    timeout: std::time::Duration::from_secs(60);
                    verify "return a handler error when QR presentation cannot be acknowledged" {
                        let (handle, _standalone) = KeyOperationCoordinator::channel();
                        let mut action = handle.begin_profile_setup(PasskeyChoice::Existing);
                        let id = action.id();
                        let mut events = action.take_events();
                        let _cancellation = action.cancellation_token();
                        let task = tokio::spawn(async move {
                            handle
                                .show_hybrid_qr_code(
                                    id,
                                    InteractionId::new(OperationId::new(1), 1),
                                    "FIDO:/test",
                                    tokio::sync::watch::channel(false).0,
                                    true,
                                )
                                .await
                        });
                        let KeyOperationEvent::HybridQrRequested { presented, .. } = events.receive().await.unwrap().1
                        else {
                            panic!("expected a QR presentation request");
                        };
                        drop(presented);
                        assert!(task.await.unwrap().is_err());
                    }
                }
            }
        }

        keyringd_testkit::scenario! {
            group "KeyOperationCoordinator::show_hybrid_qr_code" {
                case "Explicit hybrid cancellation is not a channel error" {
                    async: tokio;
                    timeout: std::time::Duration::from_secs(60);
                    verify "report explicit Hybrid cancellation as a cancellation result" {
                        let (handle, _standalone) = KeyOperationCoordinator::channel();
                        let mut action = handle.begin_profile_setup(PasskeyChoice::Existing);
                        let id = action.id();
                        let mut events = action.take_events();
                        let cancellation = action.cancellation_token();
                        let task = tokio::spawn(async move {
                            handle
                                .show_hybrid_qr_code(
                                    id,
                                    InteractionId::new(OperationId::new(1), 1),
                                    "FIDO:/test",
                                    tokio::sync::watch::channel(false).0,
                                    true,
                                )
                                .await
                        });
                        let request = events.receive().await.unwrap().1;
                        cancellation.cancel();
                        drop(request);
                        let error = task.await.unwrap().unwrap_err();
                        assert_eq!(
                            std::error::Error::source(&error)
                                .and_then(|source| source.downcast_ref::<std::io::Error>())
                                .map(std::io::Error::kind),
                            Some(std::io::ErrorKind::Interrupted)
                        );
                    }
                }
            }
        }
    }
}
