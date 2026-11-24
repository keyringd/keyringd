use super::KeyOperationCoordinator;
use keyringd_ssh_agent_protocol::{SshAgentRequest, SshAgentRequestHandler, SshAgentResponse};
use std::sync::Arc;

/// Gives each SSH signing request an action identity and an interaction route.
pub struct InteractiveAgentRequestHandler<H> {
    handler: Arc<H>,
    coordinator: Arc<KeyOperationCoordinator>,
}

impl<H> InteractiveAgentRequestHandler<H> {
    pub fn new(handler: H, coordinator: Arc<KeyOperationCoordinator>) -> Self {
        Self {
            handler: Arc::new(handler),
            coordinator,
        }
    }
}

impl<H: SshAgentRequestHandler + Send + 'static> SshAgentRequestHandler
    for InteractiveAgentRequestHandler<H>
{
    async fn handle(&self, request: SshAgentRequest) -> SshAgentResponse {
        if matches!(&request, SshAgentRequest::Sign(_)) {
            let handler = Arc::clone(&self.handler);
            let coordinator = Arc::clone(&self.coordinator);
            let (mut response, receiver) = tokio::sync::oneshot::channel();
            // The worker owns the action until cancellation and native dismissal finish.
            // Closing the client drops only the receiver, never the operation's cleanup.
            tokio::spawn(async move {
                let result = coordinator
                    .run_agent_action(async {
                        if response.is_closed() {
                            return None;
                        }
                        let operation = handler.handle(request);
                        tokio::pin!(operation);
                        tokio::select! {
                            biased;
                            _ = response.closed() => {
                                let id = keyringd_core::application::ActionId::current()
                                    .expect("an agent request owns its action");
                                coordinator.actions.cancel(id);
                                let _ = operation.await;
                                None
                            }
                            result = &mut operation => Some(result),
                        }
                    })
                    .await;
                if let Some(result) = result {
                    let _ = response.send(result);
                }
            });
            receiver
                .await
                .expect("the agent request worker must return its result")
        } else {
            self.handler.handle(request).await
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::key_operation::KeyOperationEvent;
    use keyringd_core::application::ActionId;
    use keyringd_fido_client::{
        FidoInteractionHandler, InteractionId, OperationId, OperationStatus,
    };

    keyringd_testkit::scenario! {
        group "InteractiveAgentRequestHandler::handle" {
            case "Disconnected clients retain their action through FIDO cleanup and QR dismissal" {
                async: tokio;
                timeout: std::time::Duration::from_secs(10);
                verify "cancellation waits for backend cleanup and dismissal before releasing the next action" {
                    struct Handler {
                        coordinator: Arc<KeyOperationCoordinator>,
                        started: tokio::sync::mpsc::UnboundedSender<ActionId>,
                        cleanup: Arc<tokio::sync::Notify>,
                    }
                    impl SshAgentRequestHandler for Handler {
                        async fn handle(&self, _: SshAgentRequest) -> SshAgentResponse {
                            let action = ActionId::current().unwrap();
                            self.coordinator.on_operation_status_changed(OperationStatus::OperationStarted {
                                operation_id: OperationId::new(77),
                            }).await.unwrap();
                            self.coordinator.fido_operations.set_prompt(OperationId::new(77),
                                InteractionId::new(OperationId::new(77), 1));
                            self.started.send(action).unwrap();
                            self.coordinator.actions.cancellation(action).unwrap().cancelled().await;
                            self.cleanup.notified().await;
                            SshAgentResponse::Failure
                        }
                    }
                    let (coordinator, mut events) = KeyOperationCoordinator::channel();
                    let (started, mut starts) = tokio::sync::mpsc::unbounded_channel();
                    let cleanup = Arc::new(tokio::sync::Notify::new());
                    let handler = InteractiveAgentRequestHandler::new(Handler {
                        coordinator: coordinator.clone(), started, cleanup: cleanup.clone(),
                    }, coordinator.clone());
                    let request = SshAgentRequest::Sign(keyringd_ssh_agent_protocol::SshAgentSignRequest::new(
                        keyringd_ssh_agent_protocol::SshPublicKeyBlob::from_bytes(
                            b"\0\0\0\x08ssh-test").unwrap(),
                        Vec::new(), keyringd_ssh_agent_protocol::SshAgentSignFlags::from_bits(0)));
                    let task = tokio::spawn(async move { handler.handle(request).await });
                    let id = starts.recv().await.unwrap();
                    let cancellation = coordinator.actions.cancellation(id).unwrap();
                    task.abort();
                    let _ = task.await;
                    cancellation.cancelled().await;
                    assert!(coordinator.actions.contains(id));
                    assert_eq!(coordinator.fido_operations.action_id(OperationId::new(77)), Some(id));
                    let next_coordinator = coordinator.clone();
                    let next = tokio::spawn(async move {
                        next_coordinator.run_agent_action(async { ActionId::current().unwrap() }).await
                    });
                    assert!(!next.is_finished());
                    cleanup.notify_one();
                    let (_, KeyOperationEvent::HybridQrDismissed { acknowledged, .. }) = events.receive().await.unwrap() else {
                        panic!("the old QR must be dismissed");
                    };
                    assert!(coordinator.actions.contains(id));
                    assert!(!next.is_finished());
                    acknowledged.send(()).unwrap();
                    let next_id = next.await.unwrap();
                    assert_ne!(next_id, id);
                    assert!(!coordinator.actions.contains(id));
                    assert!(coordinator.fido_operations.action_id(OperationId::new(77)).is_none());
                }
            }
        }
    }
}
