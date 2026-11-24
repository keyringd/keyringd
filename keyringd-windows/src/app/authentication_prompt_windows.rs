use keyringd_fido_client::HybridTransportUnavailableReason;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub(crate) struct AuthenticationPromptWindows {
    state: tokio::sync::watch::Sender<AuthenticationPromptStatus>,
    qr_window: Arc<Mutex<NativeQrWindowState>>,
}

#[derive(Default)]
struct NativeQrWindowState {
    prompt_id: Option<keyringd_fido_client::InteractionId>,
    dismissed: Option<(
        keyringd_fido_client::InteractionId,
        tokio::sync::oneshot::Sender<()>,
    )>,
}

impl AuthenticationPromptWindows {
    pub(crate) fn new() -> Self {
        Self {
            qr_window: Arc::new(Mutex::new(NativeQrWindowState::default())),
            state: tokio::sync::watch::channel(AuthenticationPromptStatus::Dismissed).0,
        }
    }

    pub(crate) fn show_prompt(&self, prompt_id: keyringd_fido_client::InteractionId) {
        self.state.send_replace(AuthenticationPromptStatus::Active {
            prompt_id,
            hybrid_unavailable: None,
        });
    }

    pub(crate) fn is_active(&self, prompt_id: keyringd_fido_client::InteractionId) -> bool {
        self.state.borrow().is_active(prompt_id)
    }

    pub(crate) fn hybrid_unavailable(&self, reason: HybridTransportUnavailableReason) {
        let current = *self.state.borrow();
        if let AuthenticationPromptStatus::Active { prompt_id, .. } = current {
            self.show_hybrid_availability(prompt_id, Some(reason));
        }
    }

    pub(crate) fn show_hybrid_availability(
        &self,
        prompt_id: keyringd_fido_client::InteractionId,
        hybrid_unavailable: Option<HybridTransportUnavailableReason>,
    ) {
        self.state.send_if_modified(|current| {
            if !current.is_active(prompt_id) {
                return false;
            }
            *current = AuthenticationPromptStatus::Active {
                prompt_id,
                hybrid_unavailable,
            };
            true
        });
    }

    pub(crate) fn qr_opened(&self, prompt_id: keyringd_fido_client::InteractionId) {
        self.qr_window.lock().unwrap().prompt_id = Some(prompt_id);
    }
    pub(crate) fn dismiss_hybrid(
        &self,
        prompt_id: keyringd_fido_client::InteractionId,
        acknowledged: tokio::sync::oneshot::Sender<()>,
    ) {
        let mut window = self.qr_window.lock().unwrap();
        if window.prompt_id == Some(prompt_id) {
            window.dismissed = Some((prompt_id, acknowledged));
        } else {
            let _ = acknowledged.send(());
        }
        self.state.send_if_modified(|current| {
            if !current.is_active(prompt_id) {
                return false;
            }
            *current = AuthenticationPromptStatus::Dismissed;
            true
        });
    }
    pub(crate) fn qr_closed(&self, prompt_id: keyringd_fido_client::InteractionId) {
        let mut window = self.qr_window.lock().unwrap();
        if window.prompt_id == Some(prompt_id) {
            window.prompt_id = None;
        }
        if window
            .dismissed
            .as_ref()
            .is_some_and(|(target, _)| *target == prompt_id)
        {
            let (_, acknowledged) = window.dismissed.take().unwrap();
            let _ = acknowledged.send(());
        }
    }
    pub(crate) fn dismiss(&self) {
        self.state
            .send_replace(AuthenticationPromptStatus::Dismissed);
    }

    pub(in crate::app) fn subscribe(
        &self,
    ) -> tokio::sync::watch::Receiver<AuthenticationPromptStatus> {
        self.state.subscribe()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::app) enum AuthenticationPromptStatus {
    Active {
        prompt_id: keyringd_fido_client::InteractionId,
        hybrid_unavailable: Option<HybridTransportUnavailableReason>,
    },
    Dismissed,
}

impl AuthenticationPromptStatus {
    pub(crate) fn is_active(self, expected_prompt_id: keyringd_fido_client::InteractionId) -> bool {
        matches!(
            self,
            Self::Active { prompt_id, .. } if prompt_id == expected_prompt_id
        )
    }

    pub(crate) fn hybrid_unavailable(
        self,
        expected_prompt_id: keyringd_fido_client::InteractionId,
    ) -> Option<HybridTransportUnavailableReason> {
        match self {
            Self::Active {
                prompt_id,
                hybrid_unavailable,
            } if prompt_id == expected_prompt_id => hybrid_unavailable,
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use keyringd_fido_client::{InteractionId, OperationId};
    fn id(sequence: u64) -> InteractionId {
        InteractionId::new(OperationId::new(9), sequence)
    }

    keyringd_testkit::scenario! {
        group "AuthenticationPromptWindows::show_prompt" {
            case "New request invalidates old window and cleanup only acknowledges its own qr" {
                async: tokio;
                timeout: std::time::Duration::from_secs(60);
                verify "supersede the previous request and acknowledge only the QR being closed" {
                    let windows = AuthenticationPromptWindows::new();
                    windows.show_prompt(id(1));
                    windows.qr_opened(id(1));
                    windows.show_prompt(id(2));
                    windows.qr_opened(id(2));
                    assert!(!windows.is_active(id(1)));
                    assert!(windows.is_active(id(2)));
                    let (acknowledged, mut receiver) = tokio::sync::oneshot::channel();
                    windows.dismiss_hybrid(id(2), acknowledged);
                    windows.qr_closed(id(1));
                    assert!(matches!(
                        receiver.try_recv(),
                        Err(tokio::sync::oneshot::error::TryRecvError::Empty)
                    ));
                    windows.qr_closed(id(2));
                    receiver.await.unwrap();
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "AuthenticationPromptWindows::show_prompt" {
            case "Absent qr acknowledges immediately and dismiss invalidates pin" {
                async: tokio;
                timeout: std::time::Duration::from_secs(60);
                verify "acknowledge closure of a missing QR and invalidate the active PIN prompt" {
                    let windows = AuthenticationPromptWindows::new();
                    windows.show_prompt(id(2));
                    let (acknowledged, receiver) = tokio::sync::oneshot::channel();
                    windows.dismiss_hybrid(id(2), acknowledged);
                    receiver.await.unwrap();
                    assert!(!windows.is_active(id(2)));
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "AuthenticationPromptWindows::show_prompt" {
            case "Old qr dismissal acknowledges its close without dismissing the new pin" {
                async: tokio;
                timeout: std::time::Duration::from_secs(60);
                verify "acknowledge dismissal of the older QR without clearing the newer PIN request" {
                    let windows = AuthenticationPromptWindows::new();
                    windows.show_prompt(id(1));
                    windows.qr_opened(id(1));
                    windows.show_prompt(id(2));
                    let (acknowledged, receiver) = tokio::sync::oneshot::channel();
                    windows.dismiss_hybrid(id(1), acknowledged);
                    assert!(windows.is_active(id(2)));
                    windows.qr_closed(id(1));
                    receiver.await.unwrap();
                    assert!(windows.is_active(id(2)));
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "AuthenticationPromptWindows::show_prompt" {
            case "Stale availability cannot modify current request" {
                verify "ignore availability updates carrying a superseded request ID" {
                    let windows = AuthenticationPromptWindows::new();
                    windows.show_prompt(id(1));
                    windows.show_prompt(id(2));
                    windows.show_hybrid_availability(
                        id(1),
                        Some(HybridTransportUnavailableReason::BluetoothDisabled),
                    );
                    assert_eq!(windows.state.borrow().hybrid_unavailable(id(2)), None);
                    windows.show_hybrid_availability(
                        id(2),
                        Some(HybridTransportUnavailableReason::BluetoothDisabled),
                    );
                    assert_eq!(
                        windows.state.borrow().hybrid_unavailable(id(2)),
                        Some(HybridTransportUnavailableReason::BluetoothDisabled)
                    );
                }
            }
        }
    }
}
