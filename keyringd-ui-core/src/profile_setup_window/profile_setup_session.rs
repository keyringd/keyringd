use super::{PasskeyCredentialSelectionInteraction, ProfileSetupWindowEvent};
use crate::{
    ComponentModel,
    key_operation::{
        KeyOperationCommand, KeyOperationCoordinator, KeyOperationEvent, KeyOperationState,
    },
    pin_prompt_window::{PinPromptCommand, SecurityKeyPinPrompt},
    runtime::{UiEventReceiver, UiEventSender},
};
use keyringd_core::application::ActionId;
use keyringd_core::application::ApplicationError;
use keyringd_core::application::PasskeyChoice;
use keyringd_core::application::PasskeyCredentialCandidate;
use keyringd_core::model::passkey::CredentialId;
use std::sync::{Arc, Mutex};
use tokio_util::sync::CancellationToken;

/// UI-side interaction and cancellation state for one ongoing profile setup.
pub struct ProfileSetupSession {
    operation: ProfileSetupOperation,
    cancellation: CancellationToken,
    authentication: Mutex<Authentication>,
}

enum ProfileSetupOperation {
    Create,
    Recover(PasskeyCredentialSelectionInteraction),
}

#[derive(Default)]
struct Authentication {
    state: KeyOperationState,
    pin: Option<PendingPin>,
}
struct PendingPin {
    span: tracing::Span,
    id: keyringd_fido_client::InteractionId,
    response: tokio::sync::oneshot::Sender<Option<String>>,
}
impl Authentication {
    fn cancel_pin(&mut self) {
        if let Some(pin) = self.pin.take() {
            let _ = pin.response.send(None);
        }
    }
}

impl ProfileSetupSession {
    pub async fn setup_profile<D, F>(
        self: &Arc<Self>,
        application: &keyringd_core::application::KeyringdApplication<D, F>,
        context: &D::OperationContext,
        request: keyringd_core::application::ProfileSetupRequest,
        events: UiEventSender<ProfileSetupWindowEvent>,
    ) -> Result<keyringd_core::model::Profile, keyringd_core::application::ProfileSetupFailure>
    where
        D: keyringd_core::service::DeviceKeyService + 'static,
        F: keyringd_core::service::PasskeyService + 'static,
    {
        let progress_events = events.clone();
        let progress_session = self.clone();
        let session = self.clone();
        application.setup_profile(context, &self.cancellation, request,
            move |progress| {
                progress_events.send(progress_session.progressed(progress));
            },
            move |candidates| {
                let events = events.clone();
                let session = session.clone();
                async move {
                    session.select_credential(candidates, move |candidates| {
                        events.send(crate::profile_setup_window::ProfileSetupWindowEvent::CredentialSelectionRequested(candidates));
                    }).await
                }
            },
        ).await
    }

    pub fn log_failure(failure: &keyringd_core::application::ProfileSetupFailure) {
        use keyringd_core::application::ProfileSetupFailure;
        match failure {
            ProfileSetupFailure::Failed(operation_error) => {
                operation_error.log_failure("profile setup failed")
            }
            ProfileSetupFailure::Cancelled => tracing::info!("profile setup canceled"),
            ProfileSetupFailure::PasskeyNotFound => {
                tracing::info!("profile setup did not find the Passkey")
            }
        }
    }

    pub fn new(choice: PasskeyChoice, cancellation: CancellationToken) -> Self {
        Self {
            cancellation: cancellation.clone(),
            authentication: Mutex::new(Authentication::default()),
            operation: match choice {
                PasskeyChoice::CreateNew => ProfileSetupOperation::Create,
                PasskeyChoice::Existing => ProfileSetupOperation::Recover(
                    PasskeyCredentialSelectionInteraction::new(cancellation),
                ),
            },
        }
    }

    pub fn cancel(&self) {
        self.cancellation.cancel();
        self.authentication.lock().unwrap().cancel_pin();
    }

    pub fn progressed(
        &self,
        progress: keyringd_core::application::ProfileSetupProgress,
    ) -> ProfileSetupWindowEvent {
        let mut authentication = self.authentication.lock().unwrap();
        authentication.cancel_pin();
        *authentication = Authentication::default();
        crate::profile_setup_window::ProfileSetupWindowEvent::Progressed(progress)
    }

    pub fn answer_pin(
        &self,
        request_id: keyringd_fido_client::InteractionId,
        command: PinPromptCommand,
    ) {
        let mut authentication = self.authentication.lock().unwrap();
        if self.cancellation.is_cancelled()
            || !authentication
                .pin
                .as_ref()
                .is_some_and(|pin| pin.id == request_id)
        {
            return;
        }
        let pin = authentication.pin.take().unwrap();
        let _entered = pin.span.enter();
        tracing::debug!("answering PIN interaction");
        let value = match command {
            PinPromptCommand::Submit(value) => Some(value),
            PinPromptCommand::Cancel | PinPromptCommand::Dismiss => None,
        };
        let _ = pin.response.send(value);
    }

    pub fn authentication_event(
        &self,
        coordinator: &KeyOperationCoordinator,
        flow: ActionId,
        source: ActionId,
        event: KeyOperationEvent,
    ) -> Option<ProfileSetupWindowEvent> {
        let span = coordinator.log_span(source, None, event.interaction_id());
        let _entered = span.enter();
        let terminal = matches!(
            &event,
            KeyOperationEvent::PromptDismissalRequested
                | KeyOperationEvent::HybridQrDismissed { .. }
        );
        if source != flow
            || ((!coordinator.accepts_input(Some(flow), source)
                || self.cancellation.is_cancelled())
                && !terminal)
        {
            event.reject();
            return None;
        }
        self.process_authentication(event)
    }

    fn process_authentication(&self, event: KeyOperationEvent) -> Option<ProfileSetupWindowEvent> {
        let mut authentication = self.authentication.lock().unwrap();
        let command = authentication.state.update(event)?;
        if matches!(
            &command,
            KeyOperationCommand::UpdateOperation { .. }
                | KeyOperationCommand::PresentHybridQr { .. }
                | KeyOperationCommand::ShowAuthenticatorPrompt { .. }
                | KeyOperationCommand::RequestPin { .. }
        ) {
            authentication.cancel_pin();
        }
        match command {
            KeyOperationCommand::RequestPin {
                request_id,
                reason,
                attempts_left,
                minimum_length,
                new_pin,
                response,
            } => {
                authentication.pin = Some(PendingPin {
                    span: tracing::Span::current(),
                    id: request_id,
                    response,
                });
                Some(ProfileSetupWindowEvent::PinRequested {
                    request_id,
                    prompt: SecurityKeyPinPrompt {
                        reason,
                        attempts_left,
                        minimum_length,
                        new_pin,
                    },
                })
            }
            KeyOperationCommand::CloseOperationStatus => {
                authentication.cancel_pin();
                Some(ProfileSetupWindowEvent::KeyOperation(
                    KeyOperationCommand::CloseOperationStatus,
                ))
            }
            command => Some(ProfileSetupWindowEvent::KeyOperation(command)),
        }
    }

    pub async fn receive_authentication(
        self: Arc<Self>,
        coordinator: Arc<KeyOperationCoordinator>,
        flow: ActionId,
        mut receiver: UiEventReceiver<(ActionId, KeyOperationEvent)>,
        events: UiEventSender<ProfileSetupWindowEvent>,
    ) {
        while let Some((source, event)) = receiver.receive().await {
            let span = coordinator.log_span(source, None, event.interaction_id());
            let _entered = span.enter();
            if let Some(event) = self.authentication_event(&coordinator, flow, source, event) {
                events.send(event);
            }
        }
    }

    pub async fn select_credential(
        &self,
        candidates: Vec<PasskeyCredentialCandidate>,
        publish: impl FnOnce(Vec<PasskeyCredentialCandidate>),
    ) -> Result<CredentialId, ApplicationError> {
        self.authentication.lock().unwrap().cancel_pin();
        match &self.operation {
            ProfileSetupOperation::Recover(recovery) => {
                recovery.select_credential(candidates, publish).await
            }
            ProfileSetupOperation::Create => Err(ApplicationError::InvariantViolation(
                "credential selection requires profile recovery".into(),
            )),
        }
    }

    pub fn answer_selection(&self, result: Result<CredentialId, ApplicationError>) {
        if let ProfileSetupOperation::Recover(recovery) = &self.operation {
            recovery.answer_selection(result);
        }
    }
}

impl Drop for ProfileSetupSession {
    fn drop(&mut self) {
        self.cancellation.cancel();
        if let Ok(authentication) = self.authentication.get_mut() {
            authentication.cancel_pin();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use keyringd_fido_client::OperationId;

    keyringd_testkit::scenario! {
        group "ProfileSetupSession::select_credential" {
            case "Creation never prompts for recovery selection" {
                async: tokio;
                timeout: std::time::Duration::from_secs(60);
                verify "keep the credential selector idle while creating a new Passkey" {
                    let session = ProfileSetupSession::new(PasskeyChoice::CreateNew, CancellationToken::new());
                    assert!(matches!(
                        session
                            .select_credential(vec![], |_| panic!("creation must not prompt for recovery"))
                            .await,
                        Err(ApplicationError::InvariantViolation(_))
                    ));
                }
            }
        }
    }
    fn session() -> ProfileSetupSession {
        ProfileSetupSession::new(PasskeyChoice::Existing, CancellationToken::new())
    }
    fn request_pin(
        session: &ProfileSetupSession,
        operation: u64,
    ) -> (
        keyringd_fido_client::InteractionId,
        tokio::sync::oneshot::Receiver<Option<String>>,
    ) {
        let (response, receiver) = tokio::sync::oneshot::channel();
        let Some(ProfileSetupWindowEvent::PinRequested { request_id, .. }) = session
            .process_authentication(KeyOperationEvent::PinRequested {
                request_id: keyringd_fido_client::InteractionId::new(
                    keyringd_fido_client::OperationId::new(operation),
                    1,
                ),
                reason: "PIN".into(),
                attempts_left: Some(3),
                minimum_length: Some(4),
                new_pin: false,
                response,
            })
        else {
            panic!("PIN request");
        };
        (request_id, receiver)
    }

    keyringd_testkit::scenario! {
        group "ProfileSetupSession::answer_pin" {
            case "Newer pin request rejects old answers and releases the old waiter" {
                async: tokio;
                timeout: std::time::Duration::from_secs(60);
                verify "reject the former PIN ID and release its receiver when a new request arrives" {
                    let session = session();
                    let (old, previous) = request_pin(&session, 1);
                    let (current, mut receiver) = request_pin(&session, 2);
                    assert_ne!(old, current);
                    assert_eq!(previous.await.unwrap(), None);
                    session.answer_pin(old, PinPromptCommand::Submit("old".into()));
                    assert!(matches!(
                        receiver.try_recv(),
                        Err(tokio::sync::oneshot::error::TryRecvError::Empty)
                    ));
                    session.answer_pin(current, PinPromptCommand::Submit("1234".into()));
                    assert_eq!(receiver.await.unwrap(), Some("1234".into()));
                    session.answer_pin(current, PinPromptCommand::Submit("duplicate".into()));
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "ProfileSetupSession::cancel" {
            case "Progress completion cancellation and disposal release pending pin" {
                async: tokio;
                timeout: std::time::Duration::from_secs(60);
                verify "release the PIN response channel at each terminal or advancing event" {
                    for boundary in 0..4 {
                        let session = session();
                        let (_, receiver) = request_pin(&session, 1);
                        match boundary {
                            0 => {
                                session.progressed(
                                    keyringd_core::application::ProfileSetupProgress::ProvisioningDeviceKey,
                                );
                            }
                            1 => {
                                session.process_authentication(KeyOperationEvent::PromptDismissalRequested);
                            }
                            2 => session.cancel(),
                            _ => drop(session),
                        }
                        assert_eq!(receiver.await.unwrap(), None);
                    }
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "ProfileSetupSession::cancel" {
            case "Cancelled session rejects late requests and acknowledges terminal dismissal" {
                async: tokio;
                timeout: std::time::Duration::from_secs(60);
                verify "reject post-cancellation input and still acknowledge dismissal" {
                    let (coordinator, _) = KeyOperationCoordinator::channel();
                    let action = coordinator.begin_profile_setup(PasskeyChoice::Existing);
                    let action_id = action.id();
                    let cancellation = action.cancellation_token();
                    let session = ProfileSetupSession::new(PasskeyChoice::Existing, cancellation);
                    session.cancel();
                    let (response, receiver) = tokio::sync::oneshot::channel();
                    assert!(
                        session
                            .authentication_event(
                                &coordinator,
                                action_id,
                                action_id,
                                KeyOperationEvent::PinRequested {
                                    request_id: keyringd_fido_client::InteractionId::new(
                                        keyringd_fido_client::OperationId::new(1),
                                        2
                                    ),
                                    reason: String::new(),
                                    attempts_left: None,
                                    minimum_length: None,
                                    new_pin: false,
                                    response,
                                }
                            )
                            .is_none()
                    );
                    assert_eq!(receiver.await.unwrap(), None);
                    let (acknowledged, completed) = tokio::sync::oneshot::channel();
                    let Some(ProfileSetupWindowEvent::KeyOperation(KeyOperationCommand::AcknowledgeStaleHybridDismissal(acknowledged))) = session.authentication_event(
                        &coordinator,
                        action_id,
                        action_id,
                        KeyOperationEvent::HybridQrDismissed {
                            prompt_id: keyringd_fido_client::InteractionId::new(OperationId::new(1), 1),
                            acknowledged,
                        },
                    )
                    else {
                        panic!("terminal dismissal must still be acknowledged");
                    };
                    acknowledged.send(()).unwrap();
                    completed.await.unwrap();
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "ProfileSetupSession::receive_authentication" {
            case "Authentication receiver filters foreign actions and closes with its input" {
                async: tokio;
                timeout: std::time::Duration::from_secs(60);
                verify "deliver events only for the session's action and stop when the channel closes" {
                    use keyringd_fido_client::InteractionId;
                    let (coordinator, _) = KeyOperationCoordinator::channel();
                    let action = coordinator.begin_profile_setup(PasskeyChoice::CreateNew);
                    let action_id = action.id();
                    let cancellation = action.cancellation_token();
                    let session = Arc::new(ProfileSetupSession::new(
                        PasskeyChoice::CreateNew,
                        cancellation,
                    ));
                    let (input, key_events) = UiEventSender::channel();
                    let (events, mut receiver) = UiEventSender::channel();
                    let request_id = InteractionId::new(OperationId::new(7), 2);
                    let (response, answer) = tokio::sync::oneshot::channel();
                    input.send((ActionId::new(99), KeyOperationEvent::PromptDismissalRequested));
                    input.send((
                        action_id,
                        KeyOperationEvent::PinRequested {
                            request_id,
                            reason: "Unlock the security key".into(),
                            attempts_left: Some(2),
                            minimum_length: Some(6),
                            new_pin: false,
                            response,
                        },
                    ));
                    drop(input);
                    session
                        .clone()
                        .receive_authentication(coordinator, action_id, key_events, events)
                        .await;
                    let Some(ProfileSetupWindowEvent::PinRequested {
                        request_id: delivered,
                        prompt,
                    }) = receiver.receive().await
                    else {
                        panic!("only the current action's PIN request should be delivered");
                    };
                    assert_eq!(delivered, request_id);
                    assert_eq!(
                        prompt,
                        SecurityKeyPinPrompt {
                            reason: "Unlock the security key".into(),
                            attempts_left: Some(2),
                            minimum_length: Some(6),
                            new_pin: false,
                        }
                    );
                    assert!(receiver.receive().await.is_none());
                    session.answer_pin(delivered, PinPromptCommand::Submit("123456".into()));
                    assert_eq!(answer.await.unwrap(), Some("123456".into()));
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "ProfileSetupSession::process_authentication" {
            case "Stale qr dismissal does not replace the newer shared presentation" {
                async: tokio;
                timeout: std::time::Duration::from_secs(60);
                verify "acknowledge the old QR ID while preserving its replacement" {
                    use crate::profile_setup_window::ProfileSetupInteractionScreen;
                    use crate::profile_setup_window::{ProfileSetupWindowCommand, ProfileSetupWindowState};
                    use crate::profile_setup_window::ProfileSetupEvent;
                    let session = session();
                    let mut state = ProfileSetupWindowState::default();
                    state.update(ProfileSetupEvent::NameChanged("Work".into()).into());
                    state.update(ProfileSetupEvent::SetupRequested.into());
                    for sequence in 1..=2 {
                        let (presented, shown) = tokio::sync::oneshot::channel();
                        let event = session
                            .process_authentication(KeyOperationEvent::HybridQrRequested {
                                    prompt_id: keyringd_fido_client::InteractionId::new(
                                        OperationId::new(1),
                                        sequence,
                                    ),
                                    value: format!("QR{sequence}"),
                                    context: None,
                                    security_key_available: true,
                                    presented,
                                },
                            )
                            .unwrap();
                        let Some(ProfileSetupWindowCommand::Acknowledge(presented)) = state.update(event) else {
                            panic!("QR acknowledgement");
                        };
                        presented.send(()).unwrap();
                        shown.await.unwrap();
                    }
                    let (acknowledged, completed) = tokio::sync::oneshot::channel();
                    let event = session
                        .process_authentication(KeyOperationEvent::HybridQrDismissed {
                                prompt_id: keyringd_fido_client::InteractionId::new(OperationId::new(1), 1),
                                acknowledged,
                            },
                        )
                        .unwrap();
                    let Some(ProfileSetupWindowCommand::Acknowledge(acknowledged)) = state.update(event) else {
                        panic!("stale dismissal acknowledgement");
                    };
                    acknowledged.send(()).unwrap();
                    completed.await.unwrap();
                    assert!(
                        matches!(state.interaction_presentation().screen, ProfileSetupInteractionScreen::Qr(prompt) if prompt.value == "QR2")
                    );
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "ProfileSetupSession::process_authentication" {
            case "Native password requests keep their response through the common state" {
                async: tokio;
                timeout: std::time::Duration::from_secs(60);
                verify "forward native password input through the original response channel" {
                    use crate::profile_setup_window::{ProfileSetupWindowCommand, ProfileSetupWindowState};
                    use crate::profile_setup_window::ProfileSetupEvent;
                    let session = session();
                    let mut state = ProfileSetupWindowState::default();
                    state.update(ProfileSetupEvent::NameChanged("Work".into()).into());
                    state.update(ProfileSetupEvent::SetupRequested.into());
                    let (response, receiver) = tokio::sync::oneshot::channel();
                    let event = session
                        .process_authentication(KeyOperationEvent::DeviceKeyPasswordRequested {
                                profile_name: "Work".into(),
                                response,
                            },
                        )
                        .unwrap();
                    let Some(ProfileSetupWindowCommand::KeyOperation(
                        KeyOperationCommand::RequestDeviceKeyPassword { response, .. },
                    )) = state.update(event)
                    else {
                        panic!("native password prompt");
                    };
                    response.send(Some("password".into())).unwrap();
                    assert_eq!(receiver.await.unwrap(), Some("password".into()));
                }
            }
        }
    }
}
