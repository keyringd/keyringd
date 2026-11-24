use keyringd_core::application::ActionId;
use keyringd_core::service::DeviceKeyProtectionChange;
use keyringd_ui_core::key_operation::{KeyOperationCoordinator, KeyOperationEvent};

use keyringd_ui_core::key_operation::DeviceKeyPasswordChangeInput;

#[async_trait::async_trait]
pub(crate) trait DeviceKeyInteraction: Send + Sync {
    async fn request_password(&self, profile_name: &str) -> std::io::Result<Option<String>>;
    async fn request_new_password(&self, profile_name: &str) -> std::io::Result<Option<String>>;
    async fn request_password_change(
        &self,
        profile_name: &str,
        change: DeviceKeyProtectionChange,
    ) -> std::io::Result<Option<DeviceKeyPasswordChangeInput>>;
}

#[async_trait::async_trait]
impl DeviceKeyInteraction for KeyOperationCoordinator {
    async fn request_password(&self, profile_name: &str) -> std::io::Result<Option<String>> {
        let action_id = ActionId::current().ok_or_else(inactive_error)?;
        let cancellation = self
            .actions
            .cancellation(action_id)
            .ok_or_else(inactive_error)?;
        let events = self.event_sender(action_id).ok_or_else(inactive_error)?;
        let (response, receiver) = tokio::sync::oneshot::channel();
        events
            .try_send(KeyOperationEvent::DeviceKeyPasswordRequested {
                profile_name: profile_name.to_owned(),
                response,
            })
            .map_err(|_| channel_error("password request"))?;
        tokio::select! {
            biased;
            _ = cancellation.cancelled() => {
                events.send(KeyOperationEvent::PromptDismissalRequested);
                Ok(None)
            }
            result = receiver => result.map_err(|_| channel_error("password response")),
        }
    }

    async fn request_new_password(&self, profile_name: &str) -> std::io::Result<Option<String>> {
        let action_id = ActionId::current().ok_or_else(inactive_error)?;
        let (response, receiver) = tokio::sync::oneshot::channel();
        self.event_sender(action_id)
            .ok_or_else(inactive_error)?
            .try_send(KeyOperationEvent::NewDeviceKeyPasswordRequested {
                profile_name: profile_name.to_owned(),
                response,
            })
            .map_err(|_| channel_error("new password request"))?;
        receiver
            .await
            .map_err(|_| channel_error("new password response"))
    }

    async fn request_password_change(
        &self,
        profile_name: &str,
        change: keyringd_core::service::DeviceKeyProtectionChange,
    ) -> std::io::Result<Option<keyringd_ui_core::key_operation::DeviceKeyPasswordChangeInput>>
    {
        let action_id = ActionId::current().ok_or_else(inactive_error)?;
        let (response, receiver) = tokio::sync::oneshot::channel();
        self.event_sender(action_id)
            .ok_or_else(inactive_error)?
            .try_send(KeyOperationEvent::DeviceKeyPasswordChangeRequested {
                profile_name: profile_name.to_owned(),
                change,
                response,
            })
            .map_err(|_| channel_error("password change request"))?;
        receiver
            .await
            .map_err(|_| channel_error("password change response"))
    }
}

fn inactive_error() -> std::io::Error {
    std::io::Error::new(
        std::io::ErrorKind::NotConnected,
        "device key UI session is inactive",
    )
}

fn channel_error(operation: &str) -> std::io::Error {
    std::io::Error::new(
        std::io::ErrorKind::BrokenPipe,
        format!("device key UI channel closed during {operation}"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    use keyringd_ui_core::key_operation::DeviceKeyPasswordChangeInput;

    async fn request(
        handle: &std::sync::Arc<KeyOperationCoordinator>,
        variant: u8,
    ) -> std::io::Result<Option<()>> {
        handle
            .run_agent_action(keyringd_core::test_support::run_test_execution(async {
                match variant {
                    0 => handle
                        .request_password("test")
                        .await
                        .map(|value| value.map(drop)),
                    1 => handle
                        .request_new_password("test")
                        .await
                        .map(|value| value.map(drop)),
                    _ => handle
                        .request_password_change("test", DeviceKeyProtectionChange::Enable)
                        .await
                        .map(|value| value.map(drop)),
                }
            }))
            .await
    }

    keyringd_testkit::scenario! {
        group "KeyOperationCoordinator::request_password" {
            case "Password input and user cancellation are successful responses" {
                async: tokio;
                timeout: std::time::Duration::from_secs(60);
                verify "return the entered password or a successful empty reply when the user cancels" {
                    for variant in 0..3 {
                        for cancelled in [false, true] {
                            let (handle, mut events) = KeyOperationCoordinator::channel();
                            let task = tokio::spawn(async move { request(&handle, variant).await });
                            match events.receive().await.unwrap().1 {
                                KeyOperationEvent::DeviceKeyPasswordRequested { response, .. }
                                | KeyOperationEvent::NewDeviceKeyPasswordRequested { response, .. } => {
                                    response
                                        .send((!cancelled).then(|| "password".to_owned()))
                                        .unwrap();
                                }
                                KeyOperationEvent::DeviceKeyPasswordChangeRequested { response, .. } => {
                                    assert!(
                                        response
                                            .send((!cancelled).then(|| DeviceKeyPasswordChangeInput::Enable {
                                                new_password: "password".to_owned(),
                                            }))
                                            .is_ok()
                                    );
                                }
                                _ => panic!("expected a device key password request"),
                            }
                            assert_eq!(task.await.unwrap().unwrap(), (!cancelled).then_some(()));
                        }
                    }
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "KeyOperationCoordinator::request_password" {
            case "Dropped password response is an error instead of cancellation" {
                async: tokio;
                timeout: std::time::Duration::from_secs(60);
                verify "return an interaction error when the password response sender is dropped" {
                    for variant in 0..3 {
                        let (handle, mut events) = KeyOperationCoordinator::channel();
                        let task = tokio::spawn(async move { request(&handle, variant).await });
                        let event = events.receive().await.unwrap().1;
                        drop(event);
                        assert_eq!(
                            task.await.unwrap().unwrap_err().kind(),
                            std::io::ErrorKind::BrokenPipe
                        );
                    }
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "KeyOperationCoordinator::request_password" {
            case "Disconnected password request channel is an error" {
                async: tokio;
                timeout: std::time::Duration::from_secs(60);
                verify "report a disconnected request channel as an interaction error" {
                    for variant in 0..3 {
                        let (handle, events) = KeyOperationCoordinator::channel();
                        drop(events);
                        assert_eq!(
                            request(&handle, variant).await.unwrap_err().kind(),
                            std::io::ErrorKind::BrokenPipe
                        );
                    }
                }
            }
        }
    }

    keyringd_testkit::scenario! {
            group "EsapiDeviceKeyService::generate_key" {
                case "Tpm service preserves ui failures instead of classifying them as cancellation" {
                    async: tokio;
                    timeout: std::time::Duration::from_secs(60);
                    verify "expose the underlying UI error through the TPM service" {
                        use crate::system::device_key::EsapiDeviceKeyService;
                        use keyringd_core::model::device_key::DeviceKeyAlgorithm;
    use keyringd_core::service::DeviceKeyGenerationReason;
    use keyringd_core::service::DeviceKeyGenerationRequest;
    use keyringd_core::model::device_key::LocalAuthentication;
                        use keyringd_core::service::DeviceKeyService;

                        let (handle, mut events) = KeyOperationCoordinator::channel();
                        let service = EsapiDeviceKeyService::new(
                            tss_esapi::TctiNameConf::Swtpm(Default::default()),
                            Some(handle.clone()),
                        );
                        let task = tokio::spawn(async move {
                            let result = handle
                                .run_agent_action(keyringd_core::test_support::run_test_execution(
                                    service.generate_key(
                                        &(),
                                        DeviceKeyGenerationRequest {
                                            profile_name: "test",
                                            reason: DeviceKeyGenerationReason::InitialSetup,
                                            algorithm: DeviceKeyAlgorithm::EcdsaP256,
                                            local_authentication: LocalAuthentication::Required,
                                        },
                                    ),
                                ))
                                .await;
                            let error = result
                                .err()
                                .expect("the UI failure must reject key generation");
                            assert!(!service.is_user_cancellation(&error));
                            assert!(
                                std::error::Error::source(&error)
                                    .unwrap()
                                    .is::<std::io::Error>()
                            );
                        });
                        drop(events.receive().await.unwrap().1);
                        task.await.unwrap();
                    }
                }
            }
        }
}
