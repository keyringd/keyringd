use keyringd_core::application::ApplicationError;
use keyringd_core::application::PasskeyCredentialCandidate;
use keyringd_core::model::passkey::CredentialId;
use std::sync::Mutex;
use tokio::sync::oneshot;
use tokio_util::sync::CancellationToken;

/// Publishes Passkey candidates and waits for the selected credential or cancellation.
pub(super) struct PasskeyCredentialSelectionInteraction {
    cancellation: CancellationToken,
    response: Mutex<Option<oneshot::Sender<Result<CredentialId, ApplicationError>>>>,
}

impl PasskeyCredentialSelectionInteraction {
    pub(super) fn new(cancellation: CancellationToken) -> Self {
        Self {
            cancellation,
            response: Mutex::new(None),
        }
    }

    pub(super) async fn select_credential(
        &self,
        candidates: Vec<PasskeyCredentialCandidate>,
        publish: impl FnOnce(Vec<PasskeyCredentialCandidate>),
    ) -> Result<CredentialId, ApplicationError> {
        if self.cancellation.is_cancelled() {
            return Err(ApplicationError::OperationCanceled);
        }
        let (sender, receiver) = oneshot::channel();
        {
            let mut response = self.response.lock().unwrap();
            if response
                .as_ref()
                .is_some_and(|response| !response.is_closed())
            {
                return Err(ApplicationError::InvariantViolation(
                    "credential selection is already pending".into(),
                ));
            }
            *response = Some(sender);
        }
        publish(candidates);
        let result = tokio::select! {
            biased;
            _ = self.cancellation.cancelled() => Err(ApplicationError::OperationCanceled),
            result = receiver => result.map_err(|error| ApplicationError::InvariantViolation(error.into()))?,
        };
        self.response.lock().unwrap().take();
        result
    }

    pub(super) fn answer_selection(&self, result: Result<CredentialId, ApplicationError>) {
        if let Some(response) = self.response.lock().unwrap().take() {
            let _ = response.send(result);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_util::poll;

    keyringd_testkit::scenario! {
        group "PasskeyCredentialSelectionInteraction::select_credential" {
            case "Response is bound before candidates are published" {
                async: tokio;
                timeout: std::time::Duration::from_secs(60);
                verify "let the first published candidate event receive an immediate reply" {
                    let interaction = PasskeyCredentialSelectionInteraction::new(CancellationToken::new());
                    let id = CredentialId::new(vec![1, 2, 3]);
                    let chosen = interaction
                        .select_credential(vec![], |_| {
                            interaction.answer_selection(Ok(id.clone()));
                        })
                        .await
                        .unwrap();
                    assert_eq!(chosen, id);
                    assert!(interaction.response.lock().unwrap().is_none());
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "PasskeyCredentialSelectionInteraction::select_credential" {
            case "Cancellation finishes selection without a window response" {
                async: tokio;
                timeout: std::time::Duration::from_secs(60);
                verify "resolve the pending credential request when its cancellation token fires" {
                    let cancellation = CancellationToken::new();
                    let interaction = PasskeyCredentialSelectionInteraction::new(cancellation.clone());
                    let mut selection = Box::pin(interaction.select_credential(vec![], |_| {}));
                    assert!(poll!(selection.as_mut()).is_pending());
                    cancellation.cancel();
                    assert!(matches!(
                        selection.await,
                        Err(ApplicationError::OperationCanceled)
                    ));
                    assert!(interaction.response.lock().unwrap().is_none());
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "PasskeyCredentialSelectionInteraction::select_credential" {
            case "Cancelled interaction does not publish another selection" {
                async: tokio;
                timeout: std::time::Duration::from_secs(60);
                verify "suppress credential candidate publication when the interaction is cancelled" {
                    let cancellation = CancellationToken::new();
                    cancellation.cancel();
                    let interaction = PasskeyCredentialSelectionInteraction::new(cancellation);
                    assert!(matches!(
                        interaction
                            .select_credential(vec![], |_| panic!("cancelled interaction must not prompt"))
                            .await,
                        Err(ApplicationError::OperationCanceled)
                    ));
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "PasskeyCredentialSelectionInteraction::select_credential" {
            case "Cancellation wins over an already queued answer" {
                async: tokio;
                timeout: std::time::Duration::from_secs(60);
                verify "return cancellation when cancellation and a candidate answer are both ready" {
                    let cancellation = CancellationToken::new();
                    let interaction = PasskeyCredentialSelectionInteraction::new(cancellation.clone());
                    let mut selection = Box::pin(interaction.select_credential(vec![], |_| {}));
                    assert!(poll!(selection.as_mut()).is_pending());
                    interaction.answer_selection(Ok(CredentialId::new(vec![1])));
                    cancellation.cancel();
                    assert!(matches!(
                        selection.await,
                        Err(ApplicationError::OperationCanceled)
                    ));
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "PasskeyCredentialSelectionInteraction::select_credential" {
            case "Concurrent request does not replace the pending selection" {
                async: tokio;
                timeout: std::time::Duration::from_secs(60);
                verify "preserve the original credential request when a concurrent request is rejected" {
                    let interaction = PasskeyCredentialSelectionInteraction::new(CancellationToken::new());
                    let mut selection = Box::pin(interaction.select_credential(vec![], |_| {}));
                    assert!(poll!(selection.as_mut()).is_pending());
                    assert!(matches!(
                        interaction
                            .select_credential(vec![], |_| panic!("second request must not prompt"))
                            .await,
                        Err(ApplicationError::InvariantViolation(_))
                    ));
                    let id = CredentialId::new(vec![2]);
                    interaction.answer_selection(Ok(id.clone()));
                    assert_eq!(selection.await.unwrap(), id);
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "PasskeyCredentialSelectionInteraction::select_credential" {
            case "Cancelled interaction and late answers do not affect another interaction" {
                async: tokio;
                timeout: std::time::Duration::from_secs(60);
                verify "ignore replies to the cancelled request while the next request is active" {
                    let cancellation = CancellationToken::new();
                    let old = PasskeyCredentialSelectionInteraction::new(cancellation.clone());
                    let current = PasskeyCredentialSelectionInteraction::new(CancellationToken::new());
                    let mut old_selection = Box::pin(old.select_credential(vec![], |_| {}));
                    let mut current_selection = Box::pin(current.select_credential(vec![], |_| {}));
                    assert!(poll!(old_selection.as_mut()).is_pending());
                    assert!(poll!(current_selection.as_mut()).is_pending());
                    cancellation.cancel();
                    assert!(matches!(
                        old_selection.await,
                        Err(ApplicationError::OperationCanceled)
                    ));
                    old.answer_selection(Ok(CredentialId::new(vec![1])));
                    assert!(poll!(current_selection.as_mut()).is_pending());
                    current.answer_selection(Ok(CredentialId::new(vec![2])));
                    assert_eq!(current_selection.await.unwrap(), CredentialId::new(vec![2]));
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "PasskeyCredentialSelectionInteraction::select_credential" {
            case "Dropped selection does not leave a live request" {
                async: tokio;
                timeout: std::time::Duration::from_secs(60);
                verify "release the request reservation after the selection future is dropped" {
                    let interaction = PasskeyCredentialSelectionInteraction::new(CancellationToken::new());
                    let mut selection = Box::pin(interaction.select_credential(vec![], |_| {}));
                    assert!(poll!(selection.as_mut()).is_pending());
                    drop(selection);
                    let result = interaction
                        .select_credential(vec![], |_| {
                            interaction.answer_selection(Err(ApplicationError::OperationCanceled));
                        })
                        .await;
                    assert!(matches!(result, Err(ApplicationError::OperationCanceled)));
                }
            }
        }
    }
}
