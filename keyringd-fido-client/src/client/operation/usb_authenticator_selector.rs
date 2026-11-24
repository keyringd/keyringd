use super::ctap::CtapOperationExecutor;
use super::operation_context::OperationContext;
use crate::backend::UsbHidBackend;
use crate::{AuthenticatorRequest, AuthenticatorRequirements, FidoClientError, OperationStatus};
use futures_util::{StreamExt, stream::FuturesUnordered};
use keyringd_fido::AuthenticatorTransport;
use std::sync::Arc;
use std::time::Duration;

/// Connect the USB key to check the support, and narrow down to one with the AuthenticatorSelection touch.
/// Do not start PIN/UV, registration, or authentication until the selection is complete.
pub(super) struct UsbAuthenticatorSelector<'a> {
    backend: &'a dyn UsbHidBackend,
}

impl<'a> UsbAuthenticatorSelector<'a> {
    pub(super) fn new(backend: &'a dyn UsbHidBackend) -> Self {
        Self { backend }
    }

    pub(super) async fn select(
        &self,
        operation: &OperationContext<'_>,
        requirements: &AuthenticatorRequirements,
        request: AuthenticatorRequest<'_>,
        target: Option<&crate::usb::UsbHidDeviceId>,
    ) -> Result<CtapOperationExecutor, FidoClientError> {
        loop {
            let mut descriptors = operation
                .run(self.backend.enumerate())
                .await?
                .map_err(FidoClientError::transport)?;
            if let Some(target) = target {
                descriptors.retain(|descriptor| &descriptor.id == target);
                if descriptors.is_empty() {
                    return Err(FidoClientError::NoAuthenticator);
                }
            }
            let mut connections = FuturesUnordered::new();
            for descriptor in descriptors {
                connections.push(async move {
                    // Do not stop selecting other candidates until the operation deadline because the ability check is not responding.
                    let discovery = OperationContext {
                        deadline: operation
                            .deadline
                            .min(std::time::Instant::now() + Duration::from_secs(3)),
                        ..operation.with_cancellation(operation.cancellation.child_token())
                    };
                    discovery
                        .run(async {
                            let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
                            let io = self
                                .backend
                                .open(&descriptor.id)
                                .await
                                .map_err(FidoClientError::transport)?;
                            let mut channel = crate::usb::CtapHidTransport::new(
                                io,
                                Some(Arc::new(move |status| {
                                    let _ = sender.send(status);
                                })),
                            );
                            channel
                                .initialize()
                                .await
                                .map_err(FidoClientError::transport)?;
                            CtapOperationExecutor::connect(
                                Box::new(channel),
                                AuthenticatorTransport::usb(),
                                requirements,
                                request,
                                discovery.cancellation.clone(),
                                Some(receiver),
                            )
                            .await
                            .map(|executor| executor.with_usb_device_id(descriptor.id))
                        })
                        .await?
                });
            }
            let mut candidates = Vec::new();
            let mut actionable_error = None;
            while let Some(result) = connections.next().await {
                match result {
                    Ok(candidate) => candidates.push(candidate),
                    Err(
                        error @ (FidoClientError::PinChangeRequired(_) | FidoClientError::Ctap(_)),
                    ) => actionable_error = Some(error),
                    Err(error) => tracing::debug!(?error, "USB candidate connection failed"),
                }
            }
            if operation.cancellation.is_cancelled()
                || std::time::Instant::now() >= operation.deadline
            {
                let error = if operation.cancellation.is_cancelled() {
                    FidoClientError::Cancelled
                } else {
                    FidoClientError::TimedOut
                };
                for mut candidate in candidates {
                    let _ = candidate
                        .finish::<()>(Err(FidoClientError::Cancelled))
                        .await;
                }
                return Err(error);
            }
            if !candidates.is_empty() {
                return Self::select_candidate(candidates, operation).await;
            }
            if let Some(error) = actionable_error {
                return Err(error);
            }
            operation
                .run(tokio::time::sleep(Duration::from_millis(250)))
                .await?;
        }
    }

    pub(super) async fn select_candidate(
        mut candidates: Vec<CtapOperationExecutor>,
        operation: &OperationContext<'_>,
    ) -> Result<CtapOperationExecutor, FidoClientError> {
        if candidates.len() == 1 {
            return Ok(candidates.pop().expect("one candidate"));
        }
        if candidates.is_empty() {
            return Err(FidoClientError::NoAuthenticator);
        }
        operation
            .handler
            .on_operation_status_changed(OperationStatus::AuthenticatorSelectionRequired {
                operation_id: operation.id,
                transport: AuthenticatorTransport::usb(),
            })
            .await?;
        let stop = operation.cancellation.child_token();
        let context = operation.with_cancellation(stop.clone());
        let mut selections = FuturesUnordered::new();
        for mut candidate in candidates {
            let context = &context;
            selections.push(async move {
                candidate.select(context).await?;
                Ok(candidate)
            });
        }
        let mut last_error = FidoClientError::NoAuthenticator;
        while let Some(result) = selections.next().await {
            match result {
                Ok(candidate) => {
                    stop.cancel();
                    // After the winner is decided, wait for the clean up after sending the CTAPHID_CANCEL to the other keys.
                    let cleanup = tokio::time::timeout(Duration::from_millis(250), async {
                        while let Some(result) = selections.next().await {
                            // Candidates touched at the same time are closed without starting registration/authentication.
                            match result {
                                Ok(mut other) => {
                                    if let Err(error) = other.finish(Ok(())).await {
                                        tracing::warn!(
                                            ?error,
                                            "could not close an unselected USB authenticator"
                                        );
                                    }
                                }
                                Err(FidoClientError::Cancelled) => {}
                                Err(error) => tracing::warn!(
                                    ?error,
                                    "unselected USB authenticator selection failed during cleanup"
                                ),
                            }
                        }
                    })
                    .await;
                    if let Err(error) = cleanup {
                        tracing::warn!(
                            ?error,
                            "timed out waiting for unselected USB authenticators to stop"
                        );
                    }
                    return Ok(candidate);
                }
                Err(error) => last_error = error,
            }
        }
        Err(last_error)
    }
}

#[cfg(test)]
mod tests {
    use crate::test_support::assert_fido_error;

    use super::super::test_support::*;
    use super::super::usb_authenticator_selector::UsbAuthenticatorSelector;

    use crate::*;

    use keyringd_fido::UserVerificationRequirement;
    use keyringd_fido::ctap::CtapVersion;

    use keyringd_fido::AuthenticatorTransport;
    use keyringd_fido::{CtapCommand, CtapStatusCode};

    use std::convert::Infallible;
    use std::sync::atomic::Ordering;

    use std::time::{Duration, Instant};

    keyringd_testkit::scenario! {
        group "UsbAuthenticatorSelector::select_candidate" {
            case "Only the touched key receives pin authentication and credential creation" {
                async: tokio;
                timeout: std::time::Duration::from_secs(60);
                verify "send PIN authentication and credential creation exclusively to the selected device" {
                    for (other_version, version) in [
                        (CtapVersion::Fido20, CtapVersion::Fido20),
                        (CtapVersion::Fido21Preview, CtapVersion::Fido21Preview),
                        (CtapVersion::Fido21, CtapVersion::Fido21),
                        (CtapVersion::Fido23, CtapVersion::Fido23),
                        (CtapVersion::Fido20, CtapVersion::Fido21),
                        (CtapVersion::Fido21, CtapVersion::Fido20),
                    ] {
                        let other_command = if other_version == CtapVersion::Fido20 { CtapCommand::GetAssertion } else { CtapCommand::Selection };
                        let other_status = if other_version == CtapVersion::Fido20 { CtapStatusCode::NoCredentials } else { CtapStatusCode::Ok };
                        let selection_status = if version == CtapVersion::Fido20 { CtapStatusCode::NoCredentials } else { CtapStatusCode::Ok };
                        let handler = RecordingHandler {
                            pin: Some("1234"),
                            ..RecordingHandler::default()
                        };
                        let operation = test_context(&handler, OperationId::new(70));
                        let UsbSelectionTestCandidate {
                            executor: first,
                            commands: first_commands,
                            touch: _,
                            cancelled: first_cancelled,
                        } = UsbSelectionTestCandidate::new(other_version, other_status);
                        let UsbSelectionTestCandidate {
                            executor: second,
                            commands: second_commands,
                            touch,
                            cancelled: _,
                        } = UsbSelectionTestCandidate::new(version, selection_status);
                        let selected_id = crate::usb::UsbHidDeviceId::new(b"selected-key".to_vec());
                        let second = second.with_usb_device_id(selected_id.clone());
                        let select = UsbAuthenticatorSelector::select_candidate(vec![first, second], &operation);
                        let observe = async {
                            while second_commands.lock().unwrap().is_empty() {
                                tokio::task::yield_now().await;
                            }
                            assert!(handler.existing_pin_requests.lock().unwrap().is_empty());
                            assert!(handler.new_pin_requests.lock().unwrap().is_empty());
                            assert_eq!(*first_commands.lock().unwrap(), [other_command]);
                            touch.notify_one();
                        };
                        let (selected, ()) = tokio::join!(select, observe);
                        let selected = selected.unwrap();
                        assert!(first_cancelled.load(Ordering::SeqCst));
                        let credential = selected
                            .make_credential(
                                &crate::MakeCredentialOperationContext::new(&make_options(UserVerificationRequirement::Required, 1), operation, None, None),
                            )
                            .await
                            .unwrap();
                        assert_eq!(credential.usb_device_id(), Some(&selected_id));
                        assert_eq!(handler.existing_pin_requests.lock().unwrap().len(), 1);
                        assert_eq!(*first_commands.lock().unwrap(), [other_command]);
                        let commands = second_commands.lock().unwrap();
                        assert!(commands.contains(&CtapCommand::ClientPin));
                        assert_eq!(commands.last(), Some(&CtapCommand::MakeCredential));
                    }
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "UsbAuthenticatorSelector::select_candidate" {
            case "A single candidate goes directly to pin and credential creation" {
                async: tokio;
                timeout: std::time::Duration::from_secs(60);
                verify "skip multi-device selection when exactly one authenticator qualifies" {
                    let handler = RecordingHandler {
                        pin: Some("1234"),
                        ..RecordingHandler::default()
                    };
                    let operation = test_context(&handler, OperationId::new(71));
                    let UsbSelectionTestCandidate {
                        executor: key,
                        commands,
                        touch: _,
                        cancelled: _,
                    } = UsbSelectionTestCandidate::new(CtapVersion::Fido21, CtapStatusCode::Ok);
                    let selected = UsbAuthenticatorSelector::select_candidate(vec![key], &operation)
                        .await
                        .unwrap();
                    selected
                        .make_credential(
                            &crate::MakeCredentialOperationContext::new(&make_options(UserVerificationRequirement::Required, 1), operation, None, None),
                        )
                        .await
                        .unwrap();
                    assert!(!commands.lock().unwrap().contains(&CtapCommand::Selection));
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "UsbAuthenticatorSelector::select_candidate" {
            case "Selection timeout cancels all keys without any pin attempt" {
                async: tokio;
                timeout: std::time::Duration::from_secs(60);
                verify "cancel each candidate at the deadline before requesting a PIN" {
                    for version in [CtapVersion::Fido20, CtapVersion::Fido21] {
                        let selection_command = if version == CtapVersion::Fido20 { CtapCommand::GetAssertion } else { CtapCommand::Selection };
                        let handler = RecordingHandler::default();
                        let mut operation = test_context(&handler, OperationId::new(72));
                        operation.deadline = Instant::now() + Duration::from_millis(10);
                        let UsbSelectionTestCandidate {
                            executor: first,
                            commands: first_commands,
                            touch: _,
                            cancelled: first_cancelled,
                        } = UsbSelectionTestCandidate::new(version, CtapStatusCode::Ok);
                        let UsbSelectionTestCandidate {
                            executor: second,
                            commands: second_commands,
                            touch: _,
                            cancelled: second_cancelled,
                        } = UsbSelectionTestCandidate::new(version, CtapStatusCode::Ok);
                        assert_fido_error!(
                            UsbAuthenticatorSelector::select_candidate(vec![first, second], &operation).await,
                            FidoClientError::TimedOut
                        );
                        assert!(first_cancelled.load(Ordering::SeqCst));
                        assert!(second_cancelled.load(Ordering::SeqCst));
                        assert_eq!(*first_commands.lock().unwrap(), [selection_command]);
                        assert_eq!(*second_commands.lock().unwrap(), [selection_command]);
                        assert!(handler.existing_pin_requests.lock().unwrap().is_empty());
                    }
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "UsbAuthenticatorSelector::select_candidate" {
            case "Unsupported selection does not fall back to trying pin on all keys" {
                async: tokio;
                timeout: std::time::Duration::from_secs(60);
                verify "report unsupported selection without starting PIN exchanges" {
                    let handler = RecordingHandler::default();
                    let operation = test_context(&handler, OperationId::new(73));
                    let UsbSelectionTestCandidate {
                        executor: first,
                        commands: first_commands,
                        touch: first_touch,
                        cancelled: _,
                    } = UsbSelectionTestCandidate::new(CtapVersion::Fido21, CtapStatusCode::InvalidCommand);
                    let UsbSelectionTestCandidate {
                        executor: second,
                        commands: second_commands,
                        touch: second_touch,
                        cancelled: _,
                    } = UsbSelectionTestCandidate::new(CtapVersion::Fido21, CtapStatusCode::InvalidCommand);
                    first_touch.notify_one();
                    second_touch.notify_one();
                    assert_fido_error!(
                        UsbAuthenticatorSelector::select_candidate(vec![first, second], &operation).await,
                        FidoClientError::Ctap(CtapStatusCode::InvalidCommand)
                    );
                    assert_eq!(*first_commands.lock().unwrap(), [CtapCommand::Selection]);
                    assert_eq!(*second_commands.lock().unwrap(), [CtapCommand::Selection]);
                    assert!(handler.existing_pin_requests.lock().unwrap().is_empty());
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "UsbAuthenticatorSelector::select" {
            case "An explicit target does not open other connected keys" {
                async: tokio;
                timeout: std::time::Duration::from_secs(60);
                verify "open only the device with the requested USB identifier" {
                    use crate::test_support::MockUsbHidBackend;
                    let target = crate::usb::UsbHidDeviceId::new(b"chosen-key".to_vec());
                    let other = crate::usb::UsbHidDeviceId::new(b"other-key".to_vec());
                    let mut backend = MockUsbHidBackend::new();
                    let devices = vec![other, target.clone()]
                        .into_iter()
                        .map(test_usb_device)
                        .collect::<Vec<_>>();
                    backend.expect_enumerate().returning(move || {
                        let devices = devices.clone();
                        Box::pin(async move { Ok(devices) })
                    });
                    let expected_target = target.clone();
                    backend
                        .expect_open()
                        .times(1)
                        .withf(move |id| id == &expected_target)
                        .returning(|_| {
                            Box::pin(async {
                                Err(crate::usb::UsbHidError::new(std::io::Error::other(
                                    "test connection unavailable",
                                )))
                            })
                        });
                    let handler = RecordingHandler::default();
                    let mut operation = test_context(&handler, OperationId::new(74));
                    operation.deadline = Instant::now() + Duration::from_millis(20);
                    let input = make_options(UserVerificationRequirement::Required, 1);
                    assert_fido_error!(
                        UsbAuthenticatorSelector::new(&backend)
                            .select(
                                &operation,
                                input.authenticator_requirements(),
                                crate::AuthenticatorRequest::MakeCredential(input.ctap_request()),
                                Some(&target),
                            )
                            .await,
                        FidoClientError::TimedOut
                    );
                    backend.checkpoint();
                    assert!(handler.existing_pin_requests.lock().unwrap().is_empty());
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "UsbAuthenticatorSelector::select" {
            case "A disconnected target does not fall back to another key" {
                async: tokio;
                timeout: std::time::Duration::from_secs(60);
                verify "return the target disconnect without opening a different authenticator" {
                    use crate::test_support::MockUsbHidBackend;
                    let target = crate::usb::UsbHidDeviceId::new(b"chosen-key".to_vec());
                    let mut backend = MockUsbHidBackend::new();
                    backend.expect_enumerate().returning(|| {
                        Box::pin(async {
                            Ok(vec![test_usb_device(crate::usb::UsbHidDeviceId::new(
                                b"other-key".to_vec(),
                            ))])
                        })
                    });
                    backend.expect_open().never();
                    let handler = RecordingHandler::default();
                    let operation = test_context(&handler, OperationId::new(75));
                    let input = make_options(UserVerificationRequirement::Required, 1);
                    assert_fido_error!(
                        UsbAuthenticatorSelector::new(&backend)
                            .select(
                                &operation,
                                input.authenticator_requirements(),
                                crate::AuthenticatorRequest::MakeCredential(input.ctap_request()),
                                Some(&target),
                            )
                            .await,
                        FidoClientError::NoAuthenticator
                    );
                    backend.checkpoint();
                    assert!(handler.existing_pin_requests.lock().unwrap().is_empty());
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "UsbAuthenticatorSelector::select_candidate" {
            case "Selection keepalive preserves selection instead of requesting operation approval" {
                async: tokio;
                timeout: std::time::Duration::from_secs(60);
                verify "keep the selection prompt active when a selection request reports keepalive" {
                    let handler = RecordingHandler::default();
                    let operation = test_context(&handler, OperationId::new(76));
                    let (sender, mut statuses) = tokio::sync::mpsc::unbounded_channel();
                    sender
                        .send(crate::CtapTransportStatus::UserPresenceNeeded)
                        .unwrap();
                    let transport = AuthenticatorTransport::usb();
                    let receive = operation.run_with_status(
                        &transport,
                        OperationStatus::AuthenticatorSelectionRequired {
                            operation_id: operation.id,
                            transport: AuthenticatorTransport::usb(),
                        },
                        Some(&mut statuses),
                        std::future::pending::<Result<(), Infallible>>(),
                    );
                    let observe = async {
                        while handler.events.lock().unwrap().is_empty() {
                            tokio::task::yield_now().await;
                        }
                        operation.cancellation.cancel();
                    };
                    let (result, ()) = tokio::join!(receive, observe);
                    assert_fido_error!(result, FidoClientError::Cancelled);
                    assert_eq!(*handler.events.lock().unwrap(), ["selection"]);
                }
            }
        }
    }

    fn test_usb_device(id: crate::usb::UsbHidDeviceId) -> crate::usb::UsbHidDeviceInfo {
        crate::usb::UsbHidDeviceInfo {
            id,
            vendor_id: 0,
            product_id: 0,
            manufacturer: None,
            product: None,
            serial_number: None,
        }
    }
}
