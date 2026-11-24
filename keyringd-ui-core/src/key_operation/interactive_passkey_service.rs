use super::{FidoOperationContext, KeyOperationCoordinator};
use keyringd_core::service::PasskeyService;
use keyringd_fido::webauthn::{PublicKeyCredentialCreationOptions, PublicKeyCredentialHint};
use keyringd_fido_client::{
    ClientDataProvider, FidoClient, FidoClientError, GetAssertionsResult, MakeCredentialResult,
    OperationId,
};
use std::future::Future;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Interactive FIDO routing shared by all desktop platforms. USB selection cancels
/// the Hybrid attempt and waits for cleanup before retrying.
pub struct InteractivePasskeyService {
    client: FidoClient<Arc<KeyOperationCoordinator>>,
    handler: Arc<KeyOperationCoordinator>,
}
impl InteractivePasskeyService {
    pub fn new(
        client: FidoClient<Arc<KeyOperationCoordinator>>,
        handler: Arc<KeyOperationCoordinator>,
    ) -> Self {
        Self { client, handler }
    }
    /// Runs transport attempts, sharing routing with simulated backends.
    pub async fn run<T, F, Fut, C>(
        handler: &Arc<KeyOperationCoordinator>,
        hints: &[PublicKeyCredentialHint],
        timeout: Option<u32>,
        execute: F,
        cancel: C,
    ) -> Result<T, FidoClientError>
    where
        F: Fn(bool, u32) -> Fut,
        Fut: Future<Output = Result<T, FidoClientError>> + Send,
        C: Fn(OperationId),
    {
        if hints.iter().any(|h| h.as_str() == "client-device")
            && !hints
                .iter()
                .any(|h| matches!(h.as_str(), "hybrid" | "security-key"))
        {
            return Err(FidoClientError::NoAuthenticator);
        }
        let deadline = Instant::now()
            + timeout
                .map(|ms| Duration::from_millis(u64::from(ms)))
                .unwrap_or(keyringd_fido_client::DEFAULT_TIMEOUT);
        let flow = FidoOperationContext::new(handler);
        let mut activation = flow.usb.subscribe();
        let mut usb = hints.iter().any(|h| h.as_str() == "security-key")
            && !hints.iter().any(|h| h.as_str() == "hybrid");
        let cancel_active = || {
            let id = *flow
                .active
                .lock()
                .expect("FIDO operation mutex must be unpoisoned");
            if let Some(id) = id {
                cancel(id);
            }
        };
        loop {
            if flow.cancellation.is_cancelled() {
                flow.dismiss_attempt_ui().await;
                return Err(FidoClientError::Cancelled);
            }
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                flow.dismiss_attempt_ui().await;
                return Err(FidoClientError::TimedOut);
            }
            let milliseconds = remaining.as_millis().max(1).min(u128::from(u32::MAX)) as u32;
            flow.usb.send_replace(false);
            let operation = flow.scope(execute(usb, milliseconds));
            tokio::pin!(operation);
            let result = tokio::select! {
                biased;
                _ = flow.cancellation.cancelled() => {
                    cancel_active();
                    let _ = operation.await;
                    Err(FidoClientError::Cancelled)
                }
                result = &mut operation => {
                    if matches!(result, Err(FidoClientError::OsCancelled)) && usb && !flow.cancellation.is_cancelled() {
                        flow.dismiss_attempt_ui().await;
                        usb = false;
                        continue;
                    }
                    if matches!(result, Err(FidoClientError::NoAuthenticator)) && !usb {
                        // Keep the unavailable QR visible until the app receives a choice.
                        tokio::select! {
                            biased;
                            _ = flow.cancellation.cancelled() => Err(FidoClientError::Cancelled),
                            _ = wait_for_usb(&mut activation) => { flow.dismiss_attempt_ui().await; usb = true; continue; }
                            _ = tokio::time::sleep_until(tokio::time::Instant::from_std(deadline)) => Err(FidoClientError::TimedOut),
                        }
                    } else { result }
                }
                _ = wait_for_usb(&mut activation), if !usb => {
                    cancel_active();
                    let _ = operation.await;
                    flow.dismiss_attempt_ui().await;
                    usb = true;
                    continue;
                }
            };
            flow.dismiss_attempt_ui().await;
            return if flow.cancellation.is_cancelled() {
                Err(FidoClientError::Cancelled)
            } else {
                result
            };
        }
    }
}
async fn wait_for_usb(activation: &mut tokio::sync::watch::Receiver<bool>) {
    loop {
        if *activation.borrow_and_update() {
            return;
        }
        if activation.changed().await.is_err() {
            return;
        }
    }
}
impl PasskeyService for InteractivePasskeyService {
    async fn make_credential(
        &self,
        options: PublicKeyCredentialCreationOptions,
        provider: Arc<dyn ClientDataProvider>,
    ) -> Result<MakeCredentialResult, FidoClientError> {
        Self::run(
            &self.handler,
            options.hints(),
            options.timeout(),
            |usb, timeout| {
                self.client.make_credential(
                    options
                        .clone()
                        .with_timeout(timeout)
                        .with_hints(vec![if usb {
                            PublicKeyCredentialHint::security_key()
                        } else {
                            PublicKeyCredentialHint::hybrid()
                        }]),
                    keyringd_fido_client::MakeCredentialContext::new(provider.as_ref()),
                )
            },
            |id| {
                self.client.cancel(id);
            },
        )
        .await
    }
    async fn get_assertions(
        &self,
        options: keyringd_fido::webauthn::PublicKeyCredentialRequestOptions,
        usb_device_id: Option<keyringd_fido_client::usb::UsbHidDeviceId>,
        provider: Arc<dyn ClientDataProvider>,
    ) -> Result<GetAssertionsResult, FidoClientError> {
        // The UI may retry over USB after hybrid cleanup. Keep this ceremony's data fixed.
        let client_data = provider
            .provide(options.challenge())
            .map_err(|error| FidoClientError::Protocol(Box::new(error)))?;
        Self::run(
            &self.handler,
            options.hints(),
            options.timeout(),
            |usb, timeout| {
                self.client.get_assertions(
                    options
                        .clone()
                        .with_timeout(timeout)
                        .with_hints(vec![if usb {
                            PublicKeyCredentialHint::security_key()
                        } else {
                            PublicKeyCredentialHint::hybrid()
                        }]),
                    keyringd_fido_client::GetAssertionsContext::new(&client_data)
                        .with_usb_device_id(usb_device_id.clone()),
                )
            },
            |id| {
                self.client.cancel(id);
            },
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use keyringd_core::application::ActionId;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

    fn handler() -> Arc<KeyOperationCoordinator> {
        let (handler, mut events) = KeyOperationCoordinator::channel();
        tokio::spawn(async move {
            while let Some((_flow, event)) = events.receive().await {
                match event {
                    crate::key_operation::KeyOperationEvent::HybridQrRequested {
                        presented,
                        ..
                    } => {
                        let _ = presented.send(());
                    }
                    crate::key_operation::KeyOperationEvent::HybridQrDismissed {
                        acknowledged,
                        ..
                    } => {
                        let _ = acknowledged.send(());
                    }
                    _ => {}
                }
            }
        });
        handler
    }
    async fn qr_ready(handler: &KeyOperationCoordinator) {
        use keyringd_fido_client::{FidoInteractionHandler, OperationStatus};
        handler
            .on_operation_status_changed(OperationStatus::HybridQrReady {
                operation_id: OperationId::new(1),
                qr_uri: "qr".into(),
            })
            .await
            .unwrap();
    }
    keyringd_testkit::scenario! {
        group "InteractivePasskeyService::run" {
            case "User cancel calls cancel and waits for cleanup" {
                async: tokio;
                timeout: std::time::Duration::from_secs(60);
                verify "cancel the active backend and await its cleanup before returning" {
                    let handler = handler();
                    let stop = tokio::sync::Notify::new();
                    let cleaned = AtomicBool::new(false);
                    let result = handler
                        .run_agent_action(keyringd_core::test_support::run_test_execution(
                            InteractivePasskeyService::run(
                                &handler,
                                &[],
                                Some(1000),
                                |_, _| async {
                                    use keyringd_fido_client::{FidoInteractionHandler, OperationStatus};
                                    handler
                                        .on_operation_status_changed(OperationStatus::OperationStarted {
                                            operation_id: OperationId::new(1),
                                        })
                                        .await
                                        .unwrap();
                                    handler.cancel_action(ActionId::current().unwrap());
                                    stop.notified().await;
                                    tokio::task::yield_now().await;
                                    cleaned.store(true, Ordering::SeqCst);
                                    Err::<u8, _>(FidoClientError::Cancelled)
                                },
                                |id| {
                                    assert_eq!(id, OperationId::new(1));
                                    stop.notify_one();
                                },
                            ),
                        ))
                        .await;
                    assert!(matches!(result, Err(FidoClientError::Cancelled)));
                    assert!(cleaned.load(Ordering::SeqCst));
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "InteractivePasskeyService::run" {
            case "Usb switch waits for hybrid cleanup" {
                async: tokio;
                timeout: std::time::Duration::from_secs(60);
                verify "start USB only after the first Hybrid attempt has completed cleanup" {
                    async {
                        let handler = handler();
                        let stop = tokio::sync::Notify::new();
                        let cleaned = AtomicBool::new(false);
                        let attempts = AtomicUsize::new(0);
                        let result = handler
                            .run_agent_action(keyringd_core::test_support::run_test_execution(
                                InteractivePasskeyService::run(
                                    &handler,
                                    &[],
                                    Some(1000),
                                    |usb, _| {
                                        let stop = &stop;
                                        let cleaned = &cleaned;
                                        let attempts = &attempts;
                                        let handler = &handler;
                                        async move {
                                            attempts.fetch_add(1, Ordering::SeqCst);
                                            if usb {
                                                assert!(cleaned.load(Ordering::SeqCst));
                                                return Ok(42);
                                            }
                                            use keyringd_fido_client::{
                                                FidoInteractionHandler, OperationStatus,
                                            };
                                            handler
                                                .on_operation_status_changed(OperationStatus::OperationStarted {
                                                    operation_id: OperationId::new(1),
                                                })
                                                .await
                                                .unwrap();
                                            qr_ready(handler).await;
                                            handler
                                                .fido_operations
                                                .activate_usb(ActionId::current().unwrap(), OperationId::new(1));
                                            stop.notified().await;
                                            tokio::task::yield_now().await;
                                            cleaned.store(true, Ordering::SeqCst);
                                            Err(FidoClientError::Cancelled)
                                        }
                                    },
                                    |id| {
                                        assert_eq!(id, OperationId::new(1));
                                        stop.notify_one();
                                    },
                                ),
                            ))
                            .await;
                        assert_eq!(result.unwrap(), 42);
                        assert_eq!(attempts.load(Ordering::SeqCst), 2);
                    }
                    .await;
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "InteractivePasskeyService::run" {
            case "Platform dialog cancellation returns to hybrid" {
                async: tokio;
                timeout: std::time::Duration::from_secs(60);
                verify "retry Hybrid after the operating system cancels its USB dialog" {
                    let handler = handler();
                    let attempts = AtomicUsize::new(0);
                    let result = handler
                        .run_agent_action(keyringd_core::test_support::run_test_execution(
                            InteractivePasskeyService::run(
                                &handler,
                                &[PublicKeyCredentialHint::security_key()],
                                Some(1000),
                                |usb, _| {
                                    let attempts = &attempts;
                                    async move {
                                        if attempts.fetch_add(1, Ordering::SeqCst) == 0 {
                                            assert!(usb);
                                            Err(FidoClientError::OsCancelled)
                                        } else {
                                            assert!(!usb);
                                            Ok(42)
                                        }
                                    }
                                },
                                |_| panic!("completed attempts need no cancellation"),
                            ),
                        ))
                        .await;
                    assert_eq!(result.unwrap(), 42);
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "InteractivePasskeyService::run" {
            case "Unavailable hybrid keeps app usb action available" {
                async: tokio;
                timeout: std::time::Duration::from_secs(60);
                verify "leave the USB action available when the Hybrid transport cannot start" {
                    async {
                        let handler = handler();
                        let result = handler
                            .run_agent_action(keyringd_core::test_support::run_test_execution(
                                InteractivePasskeyService::run(
                                    &handler,
                                    &[],
                                    Some(1000),
                                    |usb, _| {
                                        let handler = &handler;
                                        async move {
                                            if usb {
                                                return Ok(42);
                                            }
                                            use keyringd_fido_client::{
                                                FidoInteractionHandler, OperationStatus,
                                            };
                                            handler
                                                .on_operation_status_changed(OperationStatus::OperationStarted {
                                                    operation_id: OperationId::new(1),
                                                })
                                                .await
                                                .unwrap();
                                            qr_ready(handler).await;
                                            handler
                                                .fido_operations
                                                .activate_usb(ActionId::current().unwrap(), OperationId::new(1));
                                            Err(FidoClientError::NoAuthenticator)
                                        }
                                    },
                                    |_| panic!("completed attempts need no cancellation"),
                                ),
                            ))
                            .await;
                        assert_eq!(result.unwrap(), 42);
                    }
                    .await;
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "InteractivePasskeyService::run" {
            case "User cancellation ends the whole request" {
                async: tokio;
                timeout: std::time::Duration::from_secs(60);
                verify "return cancellation for the request without starting a new attempt" {
                    for usb in [false, true] {
                        for outcome in 0..3 {
                            let handler = handler();
                            let attempts = AtomicUsize::new(0);
                            let hints = if usb { vec![PublicKeyCredentialHint::security_key()] } else { vec![] };
                            let result = handler.run_agent_action(
                                keyringd_core::test_support::run_test_execution(
                                    InteractivePasskeyService::run(
                                        &handler, &hints, Some(1000),
                                        |_, _| async {
                                            attempts.fetch_add(1, Ordering::SeqCst);
                                            handler.cancel_action(ActionId::current().unwrap());
                                            match outcome {
                                                0 => Err(FidoClientError::NoAuthenticator),
                                                1 => Err(FidoClientError::OsCancelled),
                                                _ => Ok(42),
                                            }
                                        },
                                        |_| {},
                                    ),
                                ),
                            ).await;
                            assert!(matches!(result, Err(FidoClientError::Cancelled)));
                            assert_eq!(attempts.load(Ordering::SeqCst), 1);
                        }
                    }
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "InteractivePasskeyService::run" {
            case "Client device only is not replaced with hybrid" {
                async: tokio;
                timeout: std::time::Duration::from_secs(60);
                verify "preserve the client-device hint for an internal authenticator request" {
                    let handler = handler();
                    let result = handler
                        .run_agent_action(keyringd_core::test_support::run_test_execution(
                            InteractivePasskeyService::run(
                                &handler,
                                &[PublicKeyCredentialHint::client_device()],
                                Some(1000),
                                |_, _| async {
                                    panic!("unsupported hints must not start another route");
                                    #[allow(unreachable_code)]
                                    Ok::<u8, FidoClientError>(42)
                                },
                                |_| {},
                            ),
                        ))
                        .await;
                    assert!(matches!(result, Err(FidoClientError::NoAuthenticator)));
                }
            }
        }
    }
}
