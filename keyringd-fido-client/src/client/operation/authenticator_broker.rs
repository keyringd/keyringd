use super::operation_context::OperationContext;
use crate::backend::UsbBackend;

use crate::{
    FidoClientError, GetAssertionOperationContext, GetAssertionsResult, HybridChannelConnector,
    MakeCredentialOperationContext, MakeCredentialResult, TransportSelection,
};

use super::HybridOperationExecutor;
use super::UsbOperationExecutor;

/// The operations are sorted to the set route and do not fall back to the unconnected route.
#[derive(Default)]
pub(crate) struct AuthenticatorBroker {
    hybrid: Option<HybridOperationExecutor>,
    usb: Option<UsbOperationExecutor>,
}
impl AuthenticatorBroker {
    async fn select_transport(
        selection: Option<TransportSelection>,
        operation: &OperationContext<'_>,
    ) -> Result<TransportSelection, FidoClientError> {
        if let Some(selection) = selection {
            return Ok(selection);
        }
        let request = crate::TransportInteraction::new(crate::InteractionId::new(operation.id, 0));
        let response = operation
            .run(
                request
                    .id()
                    .scope(operation.handler.request_transport(request)),
            )
            .await??;
        if response.id() != request.id() {
            return Err(FidoClientError::protocol(std::io::Error::other(
                "transport interaction ID mismatch",
            )));
        }
        match response.into_value() {
            Some(transport) if transport == keyringd_fido::AuthenticatorTransport::usb() => {
                Ok(TransportSelection::Usb)
            }
            Some(transport) if transport == keyringd_fido::AuthenticatorTransport::hybrid() => {
                Ok(TransportSelection::Hybrid)
            }
            Some(transport) if transport == keyringd_fido::AuthenticatorTransport::internal() => {
                Ok(TransportSelection::Internal)
            }
            Some(_) => Err(FidoClientError::UnsupportedRequirement(
                "select USB, Hybrid, or Internal transport".into(),
            )),
            None => Err(FidoClientError::Cancelled),
        }
    }

    #[cfg(test)]
    pub(crate) fn new(
        hybrid_connector: std::sync::Arc<dyn HybridChannelConnector>,
        usb_backend: UsbBackend,
    ) -> Self {
        Self {
            hybrid: Some(HybridOperationExecutor::new(hybrid_connector)),
            usb: Some(UsbOperationExecutor::new(usb_backend)),
        }
    }
    pub(crate) fn set_usb_backend(&mut self, backend: UsbBackend) {
        self.usb = Some(UsbOperationExecutor::new(backend));
    }
    pub(crate) fn set_hybrid_connector(
        &mut self,
        connector: std::sync::Arc<dyn HybridChannelConnector>,
    ) {
        self.hybrid = Some(HybridOperationExecutor::new(connector));
    }
    // Keep transport futures on the heap for GUI worker threads.
    pub(crate) async fn make_credential<A: crate::InternalAuthenticator>(
        &self,
        context: MakeCredentialOperationContext<'_>,
        internal: &A,
    ) -> Result<MakeCredentialResult, FidoClientError> {
        Box::pin(async {
            match Self::select_transport(context.transport_selection, &context.operation).await? {
                TransportSelection::Hybrid => {
                    self.hybrid
                        .as_ref()
                        .ok_or(FidoClientError::NoAuthenticator)?
                        .make_credential(context)
                        .await
                }
                TransportSelection::Usb => {
                    self.usb
                        .as_ref()
                        .ok_or(FidoClientError::NoAuthenticator)?
                        .make_credential(context)
                        .await
                }
                TransportSelection::Internal => {
                    let mut request = context.prepared.ctap_request().clone();
                    if context.prepared.user_verification()
                        == &keyringd_fido::UserVerificationRequirement::Required
                    {
                        request = request.with_option("uv", true);
                    }
                    let stop = context.operation.cancellation.child_token();
                    let response = internal.make_credential(&request, stop.clone());
                    tokio::pin!(response);
                    let response = match context.operation.run(&mut response).await {
                        Ok(result) => result?,
                        Err(error) => {
                            stop.cancel();
                            let _ = response.await;
                            return Err(error);
                        }
                    };
                    super::ctap::make_credential_result(
                        response,
                        context.prepared.client_data.as_bytes(),
                        keyringd_fido::AuthenticatorTransport::internal(),
                    )
                }
            }
        })
        .await
    }

    pub(crate) async fn get_assertions<A: crate::InternalAuthenticator>(
        &self,
        context: GetAssertionOperationContext<'_>,
        internal: &A,
    ) -> Result<GetAssertionsResult, FidoClientError> {
        Box::pin(async {
            let transport =
                Self::select_transport(context.transport_selection, &context.operation).await?;
            if transport != TransportSelection::Internal
                && context
                    .prepared
                    .ctap_request()
                    .options()
                    .is_some_and(|options| options.get("up") == Some(&false))
            {
                return Err(FidoClientError::UnsupportedRequirement(
                    "disabling user presence is supported only on the Internal route".into(),
                ));
            }
            match transport {
                TransportSelection::Hybrid => {
                    self.hybrid
                        .as_ref()
                        .ok_or(FidoClientError::NoAuthenticator)?
                        .get_assertions(context)
                        .await
                }
                TransportSelection::Usb => {
                    self.usb
                        .as_ref()
                        .ok_or(FidoClientError::NoAuthenticator)?
                        .get_assertions(context)
                        .await
                }
                TransportSelection::Internal => {
                    let mut request = context.prepared.ctap_request().clone();
                    if context.prepared.user_verification()
                        == &keyringd_fido::UserVerificationRequirement::Required
                    {
                        request = request.with_option("uv", true);
                    }
                    let stop = context.operation.cancellation.child_token();
                    let response = internal.get_assertion(&request, stop.clone());
                    tokio::pin!(response);
                    let response = match context.operation.run(&mut response).await {
                        Ok(result) => result?,
                        Err(error) => {
                            stop.cancel();
                            // Keep the operation active until native authentication and UI settle.
                            let _ = response.await;
                            return Err(error);
                        }
                    };
                    GetAssertionsResult::from_ctap(
                        vec![response],
                        context.prepared.client_data.clone(),
                        keyringd_fido::AuthenticatorTransport::internal(),
                        request.allow_list(),
                    )
                }
            }
        })
        .await
    }
}

#[cfg(test)]
mod tests {

    use super::super::ctap::CtapUserVerification;
    use super::super::operation_context::OperationContext;
    use super::super::test_support::*;
    use crate::client::PreparedMakeCredential;
    use crate::test_support::MockCtapTransport;
    use crate::test_support::MockHybridChannelConnector;

    use std::collections::BTreeMap;
    use std::convert::Infallible;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, Instant};

    use keyringd_fido::UserVerificationRequirement;

    use super::super::ctap::CtapOperationExecutor;

    use super::super::operation_manager::OperationManager;
    use super::*;
    use crate::*;

    use keyringd_fido::AuthenticatorTransport;

    use crate::ClientData;
    use crate::test_support::assert_fido_error;

    keyringd_testkit::scenario! {
        group "CtapUserVerification::apply_make_user_verification" {
            case "Pin setup requests a new pin and reuses it for authentication" {
                async: tokio;
                timeout: std::time::Duration::from_secs(60);
                verify "collect the new PIN once and use it for the next PIN-authenticated request" {
                    let handler = RecordingHandler {
                        pin: Some("123456"),
                        ..RecordingHandler::default()
                    };
                    let model = Arc::new(keyringd_test_authenticator::ModelCtapAuthenticator::new());
                    let mut verification_client = crate::CtapClient::new(Box::new(built_in_uv_transport(
                        model,
                        crate::ctap::test_support::TestPinUvKeyAgreement::new(PinUvAuthProtocol::Two),
                        Arc::new(Mutex::new(Vec::new())),
                        0,
                        keyringd_fido::CtapStatusCode::UserVerificationInvalid,
                        8,
                    )));
                    let verification_transport = AuthenticatorTransport::usb();
                    let verification_capabilities = AuthenticatorCapabilities::from_get_info(
                        model_capabilities()
                            .info()
                            .clone()
                            .with_options(BTreeMap::from([
                                ("rk".to_owned(), true),
                                ("clientPin".to_owned(), false),
                                ("pinUvAuthToken".to_owned(), true),
                            ]))
                            .with_pin_uv_auth_protocols(vec![2])
                            .with_min_pin_length(6),
                    );
                    let mut authenticator = CtapUserVerification::new(
                        &mut verification_client,
                        &verification_capabilities,
                        &verification_transport,
                        &UserVerificationRequirement::Required,
                    )
                    .unwrap();
                    let context = test_context(&handler, OperationId::new(30));
                    let mut request = make_options(UserVerificationRequirement::Required, 1)
                        .ctap_request()
                        .clone();
                    authenticator
                        .apply_make_user_verification(&context, &mut request)
                        .await
                        .expect("new PIN is configured and used to obtain a token");
                    assert!(handler.existing_pin_requests.lock().unwrap().is_empty());
                    {
                        let requests = handler.new_pin_requests.lock().unwrap();
                        assert_eq!(requests.len(), 1);
                        assert_eq!(requests[0].minimum_length(), 6);
                        assert!(!requests[0].rejected_by_authenticator());
                    }
                    let verification_capabilities = AuthenticatorCapabilities::from_get_info(
                        model_capabilities()
                            .info()
                            .clone()
                            .with_options(BTreeMap::from([
                                ("rk".to_owned(), true),
                                ("clientPin".to_owned(), true),
                                ("pinUvAuthToken".to_owned(), true),
                            ]))
                            .with_pin_uv_auth_protocols(vec![2]),
                    );
                    let mut authenticator = CtapUserVerification::new(
                        &mut verification_client,
                        &verification_capabilities,
                        &verification_transport,
                        &UserVerificationRequirement::Required,
                    )
                    .unwrap();
                    authenticator
                        .apply_make_user_verification(&context, &mut request)
                        .await
                        .expect("configured PIN is requested through the existing PIN handler");
                    assert_eq!(handler.existing_pin_requests.lock().unwrap().len(), 1);
                    assert_eq!(handler.new_pin_requests.lock().unwrap().len(), 1);
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "AuthenticatorBroker::make_credential" {
            case "Security Key routing does not start Hybrid" {
                async: tokio;
                timeout: std::time::Duration::from_secs(5);
                given "Prepare a Confirmed Context and an Unstarted Hybrid Connector to Security Key" {
                    let entered = Arc::new(AtomicBool::new(false));
                    let dropped = Arc::new(AtomicBool::new(false));
                    let connector = Arc::new(tracked_hybrid_connector(Arc::clone(&entered), dropped));
                    let handler = Arc::new(RecordingHandler::default());
                    let service = BrokerTestClient::with_hybrid_connector(Arc::clone(&handler), connector);
                    let options = make_options(UserVerificationRequirement::Preferred, 1);
                    let options = PreparedMakeCredential::new(
                        options.options.with_hints(vec![
                            keyringd_fido::webauthn::PublicKeyCredentialHint::security_key(),
                        ]),
                        (Arc::new(|_: &[u8]| Ok(ClientData::from_bytes(vec![1])))).as_ref(),
                    )
                    .unwrap();
                }
                when "Start the next MakeCredential" {
                    let outcome = service.make_credential(options).await;
                }
                then "Don't create Hybrid Prompt by waiting only on USB" {
                    assert_fido_error!(outcome, FidoClientError::TimedOut);
                    assert!(!entered.load(Ordering::SeqCst));
                    assert!(handler.qr_uris.lock().unwrap().is_empty());
                }
            }
            case "Hybrid routing does not present switching to Security Key" {
                async: tokio;
                timeout: std::time::Duration::from_secs(5);
                given "Prepare a Hybrid Connector that has been confirmed to Hybrid and waits for a connection" {
                    let entered = Arc::new(AtomicBool::new(false));
                    let dropped = Arc::new(AtomicBool::new(false));
                    let connector = Arc::new(tracked_hybrid_connector(Arc::clone(&entered), dropped));
                    let handler = Arc::new(RecordingHandler::default());
                    let service = BrokerTestClient::with_hybrid_connector(Arc::clone(&handler), connector);
                    let options = make_options(UserVerificationRequirement::Preferred, 1);
                    let options = PreparedMakeCredential::new(
                        options.options.with_hints(vec![
                            keyringd_fido::webauthn::PublicKeyCredentialHint::hybrid(),
                        ]),
                        (Arc::new(|_: &[u8]| Ok(ClientData::from_bytes(vec![1])))).as_ref(),
                    )
                    .unwrap();
                    let deadline = Instant::now() + Duration::from_millis(20);
                }
                when "Start the next MakeCredential and wait until Deadline" {
                    let result = service
                        .make_credential_with_deadline(options, deadline)
                        .await;
                }
                then "Presenting an immutable QR Prompt waiting only for Hybrid" {
                    assert_fido_error!(result, FidoClientError::TimedOut);
                    assert!(entered.load(Ordering::SeqCst));
                    assert_eq!(handler.qr_uris.lock().unwrap().len(), 1);
                }
            }
        }
    }
    keyringd_testkit::scenario! {
        group "AuthenticatorBroker::make_credential" {
            case "Drops the Hybrid connection future before reporting a timeout" {
                why "The pending Hybrid connection future must be destroyed before timeout is reported so its transport resources cannot outlive the operation."
                async: tokio;
                timeout: std::time::Duration::from_secs(5);
                given "Prepare an uncompleted Hybrid Transport connection Future and 10ms Operation Deadline" {
                    let entered = Arc::new(AtomicBool::new(false));
                    let dropped = Arc::new(AtomicBool::new(false));
                    let connector = Arc::new(tracked_hybrid_connector(
                        Arc::clone(&entered),
                        Arc::clone(&dropped),
                    ));
                    let handler = Arc::new(RecordingHandler::default());
                    let service = BrokerTestClient::with_hybrid_connector(Arc::clone(&handler), connector);
                    let options = make_options(UserVerificationRequirement::Discouraged, 1);
                    let deadline = Instant::now() + Duration::from_millis(10);
                }
                when "Coordinator waits for Hybrid Transport connection beyond Deadline" {
                    let result = service
                        .make_credential_with_deadline(options, deadline)
                        .await;
                }
                then "Returns Timeout and waits, Drop Connection Future to complete Lifecycle only once" {
                    assert_fido_error!(result, FidoClientError::TimedOut);
                    assert!(entered.load(Ordering::SeqCst));
                    assert!(dropped.load(Ordering::SeqCst));
                    assert_eq!(
                        *handler.events.lock().unwrap(),
                        ["started", "progress", "backend_finished"]
                    );
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "AuthenticatorBroker::make_credential" {
            case "Drops the Hybrid connection before completing a canceled operation" {
                why "Cancellation must destroy an in-progress Hybrid connection before completing the operation to prevent late connection side effects."
                async: tokio;
                timeout: std::time::Duration::from_secs(5);
                given "Prepare a registered operation with a Hybrid Transport connection Future that never completes" {
                    let entered = Arc::new(AtomicBool::new(false));
                    let dropped = Arc::new(AtomicBool::new(false));
                    let connector = Arc::new(tracked_hybrid_connector(
                        Arc::clone(&entered),
                        Arc::clone(&dropped),
                    ));
                    let handler = Arc::new(RecordingHandler::default());
                    let service = Arc::new(BrokerTestClient::with_hybrid_connector(
                        Arc::clone(&handler),
                        connector,
                    ));
                    let options = make_options(UserVerificationRequirement::Discouraged, 1);
                    let deadline = Instant::now() + Duration::from_secs(1);
                    let operation_service = Arc::clone(&service);
                    let operation = tokio::spawn(async move {
                        operation_service
                            .make_credential_with_deadline(options, deadline)
                            .await
                    });
                }
                when "Cancel Operations after Coordinator starts waiting for connection" {
                    while !entered.load(Ordering::SeqCst) {
                        tokio::task::yield_now().await;
                    }
                    let cancelled = service.cancel(OperationId::new(1));
                    let result = operation.await.unwrap();
                }
                then "Returns Cancel and Waiting to Drop Connection Future to complete Lifecycle only once" {
                    assert!(cancelled);
                    assert_fido_error!(result, FidoClientError::Cancelled);
                    assert!(dropped.load(Ordering::SeqCst));
                    assert_eq!(
                        *handler.events.lock().unwrap(),
                        ["started", "progress", "backend_finished"]
                    );
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "OperationManager::run_operation" {
            case "Completes and dismisses an operation once on error" {
                why "Every started operation must be completed and dismissed exactly once on error to avoid leaked UI state and duplicate callbacks."
                async: tokio;
                timeout: std::time::Duration::from_secs(5);
                given "Prepare RecordingHandler, OperationManager, FidoclEnterror" {
                    let handler = std::sync::Arc::new(RecordingHandler::default());
                }
                when "Generate an OperationManager with RecordingHandler" {
                    let service = BrokerTestClient::new(std::sync::Arc::clone(&handler));
                }
                then "Verify that the results match FidoClientError::UnsupportedRequirement" {
                    assert_fido_error!(
                        service
                            .run_operation(Instant::now() + Duration::from_secs(1), |_| async {
                                Err::<MakeCredentialResult, _>(FidoClientError::UnsupportedRequirement(
                                    "test requirement".into(),
                                ))
                            })
                            .await,
                        FidoClientError::UnsupportedRequirement(_)
                    );
                    assert_eq!(
                        *handler.events.lock().unwrap(),
                        ["started", "backend_finished"]
                    );
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "OperationManager::run_operation" {
            case "Completes and dismisses successful and canceled operations once each" {
                why "Success and cancellation must each close the public operation lifecycle exactly once, independent of protocol response semantics."
                async: tokio;
                timeout: std::time::Duration::from_secs(5);
                given "Prepare an Operation for Success and Cancel Results" {
                    let outcomes = [Ok(7), Err(FidoClientError::Cancelled)];
                }
                when "Complete each operation" {
                    let mut results = Vec::new();
                    for result in outcomes {
                        let handler = std::sync::Arc::new(RecordingHandler::default());
                        let service = BrokerTestClient::new(std::sync::Arc::clone(&handler));
                        let actual = service
                            .run_operation(Instant::now() + Duration::from_secs(1), |_| async {
                                result
                            })
                            .await;
                        let cancelled = matches!(&actual, Err(FidoClientError::Cancelled));
                        results.push((
                            actual.ok(),
                            cancelled,
                            handler.events.lock().unwrap().clone(),
                        ));
                    }
                }
                then "Both notify Finished and Dismissed only once" {
                    assert_eq!(
                        results[0],
                        (Some(7), false, vec!["started", "backend_finished"])
                    );
                    assert_eq!(
                        results[1],
                        (None, true, vec!["started", "backend_finished"])
                    );
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "OperationContext::run_with_status" {
            case "Forwards channel status without extending the deadline" {
                why "Forwarding transport status must not refresh the ceremony deadline, otherwise a noisy channel could keep an operation alive indefinitely."
                async: tokio;
                timeout: std::time::Duration::from_secs(5);
                given "Prepare RecordingHandler, CtapTransportStatus, OperationID" {
                    let handler = RecordingHandler::default();
                    let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel();
                    sender.send(crate::CtapTransportStatus::Processing).unwrap();
                    sender
                        .send(crate::CtapTransportStatus::UserPresenceNeeded)
                        .unwrap();
                }
                when "Call EXECUTE_WITH_STATUS" {
                    let result = test_context(&handler, OperationId::new(18))
                        .run_with_status(
                            &AuthenticatorTransport::usb(),
                            OperationStatus::UserPresenceRequired {
                                operation_id: OperationId::new(18),
                                transport: AuthenticatorTransport::usb(),
                            },
                            Some(&mut receiver),
                            async {
                                tokio::time::sleep(Duration::from_millis(10)).await;
                                Ok::<_, Infallible>(9)
                            },
                        )
                        .await
                        .unwrap()
                        .unwrap();
                }
                then "Verify that the results match FidoClientError::TimedOut" {
                    assert_eq!(result, 9);
                    assert_eq!(*handler.events.lock().unwrap(), ["progress", "progress"]);
                    let (_sender, mut receiver) = tokio::sync::mpsc::unbounded_channel();
                    let timed_out = OperationContext {
                        deadline: Instant::now(),
                        ..test_context(&handler, OperationId::new(19))
                    }
                    .run_with_status(
                        &AuthenticatorTransport::usb(),
                        OperationStatus::UserPresenceRequired {
                            operation_id: OperationId::new(18),
                            transport: AuthenticatorTransport::usb(),
                        },
                        Some(&mut receiver),
                        std::future::pending::<Result<(), Infallible>>(),
                    )
                    .await;
                    assert_fido_error!(timed_out, FidoClientError::TimedOut);
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "CtapOperationExecutor::finish" {
            case "Cleans up the channel according to the operation result" {
                why "Channel cleanup differs between reusable success and terminal failure, and this resource-ownership invariant is not defined by CTAP or WebAuthn."
                async: tokio;
                timeout: std::time::Duration::from_secs(5);
                given "Prepare a Channel for Success, CTAP Failure, and Cancel" {
                    let success_channel = cleanup_transport(false);
                    let mut success = CtapOperationExecutor::new(
                        success_channel,
                        AuthenticatorTransport::usb(),
                        model_capabilities(),
                    );
                    let failed_channel = cleanup_transport(false);
                    let mut failed = CtapOperationExecutor::new(
                        failed_channel,
                        AuthenticatorTransport::usb(),
                        model_capabilities(),
                    );
                    let cancelled_channel = cleanup_transport(true);
                    let mut cancelled = CtapOperationExecutor::new(
                        cancelled_channel,
                        AuthenticatorTransport::usb(),
                        model_capabilities(),
                    );
                }
                when "Execute the channel termination process on each result" {
                    let success_result = success.finish(Ok::<_, FidoClientError>(7)).await;
                    let failed_result = failed
                        .finish::<()>(Err(FidoClientError::Ctap(
                            keyringd_fido::CtapStatusCode::OperationDenied,
                        )))
                        .await;
                    let cancelled_result = cancelled
                        .finish::<()>(Err(FidoClientError::Cancelled))
                        .await;
                }
                then "Close for success and CTAP failure and CANCEL processing only when CANCEL" {
                    assert_eq!(success_result.unwrap(), 7);

                    let error = failed_result.expect_err("CTAP failure must be retained");
                    assert!(matches!(
                        &error,
                        FidoClientError::Ctap(keyringd_fido::CtapStatusCode::OperationDenied)
                    ));

                    assert_fido_error!(cancelled_result, FidoClientError::Cancelled);
                }
            }
        }
    }
    keyringd_testkit::scenario! {
        group "CtapOperationExecutor::finish" {
            case "Retains pin error when close fails" {
                async: tokio;
                timeout: std::time::Duration::from_secs(60);
                verify "return the PIN error after the fallback channel close fails" {
                    let mut executor = CtapOperationExecutor::new(
                        failing_cleanup_transport(false),
                        AuthenticatorTransport::usb(),
                        model_capabilities(),
                    );

                    let result = executor
                        .finish::<()>(Err(FidoClientError::Ctap(
                            keyringd_fido::CtapStatusCode::PinInvalid,
                        )))
                        .await;

                    assert_fido_error!(
                        result,
                        FidoClientError::Ctap(keyringd_fido::CtapStatusCode::PinInvalid)
                    );
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "OperationManager::run_operation" {
            case "Completion failures do not replace the operation error" {
                async: tokio;
                timeout: std::time::Duration::from_secs(60);
                verify "return the original operation error when notifying completion also fails" {
                    let manager = OperationManager::new(RecordingHandler {
                        fail_completion: true,
                        ..Default::default()
                    });
                    let result = manager
                        .run_operation(Instant::now() + Duration::from_secs(5), |_| async {
                            Err::<(), _>(FidoClientError::Ctap(
                                keyringd_fido::CtapStatusCode::PinInvalid,
                            ))
                        })
                        .await;
                    assert_fido_error!(
                        result,
                        FidoClientError::Ctap(keyringd_fido::CtapStatusCode::PinInvalid)
                    );
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "OperationManager::run_operation" {
            case "Successful operations report completion failures" {
                async: tokio;
                timeout: std::time::Duration::from_secs(60);
                verify "return the completion notification error after otherwise successful execution" {
                    let manager = OperationManager::new(RecordingHandler {
                        fail_completion: true,
                        ..Default::default()
                    });
                    let result = manager
                        .run_operation(Instant::now() + Duration::from_secs(5), |_| async { Ok(7) })
                        .await;
                    assert_fido_error!(result, FidoClientError::Interaction(_));
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "CtapOperationExecutor::finish" {
            case "Retains cancellation and timeout when cancel fails" {
                async: tokio;
                timeout: std::time::Duration::from_secs(60);
                verify "preserve the original cancellation or deadline error after a failed cancel request" {
                    for timed_out in [false, true] {
                        let mut executor = CtapOperationExecutor::new(
                            failing_cleanup_transport(true),
                            AuthenticatorTransport::usb(),
                            model_capabilities(),
                        );
                        let error = if timed_out {
                            FidoClientError::TimedOut
                        } else {
                            FidoClientError::Cancelled
                        };

                        let result = executor.finish::<()>(Err(error)).await;

                        if timed_out {
                            assert_fido_error!(result, FidoClientError::TimedOut);
                        } else {
                            assert_fido_error!(result, FidoClientError::Cancelled);
                        }
                    }
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "CtapOperationExecutor::finish" {
            case "Reports cleanup failure after successful operation" {
                async: tokio;
                timeout: std::time::Duration::from_secs(60);
                verify "return a cleanup error after the authenticator operation succeeds" {
                    let mut executor = CtapOperationExecutor::new(
                        failing_cleanup_transport(false),
                        AuthenticatorTransport::usb(),
                        model_capabilities(),
                    );

                    let error = executor.finish(Ok(7)).await.unwrap_err();

                    assert!(matches!(&error, FidoClientError::Transport(_)));
                    let source = std::error::Error::source(&error).unwrap();
                    assert!(source.to_string().contains("cleanup failure"));
                }
            }
        }
    }

    fn failing_cleanup_transport(cancelled: bool) -> MockCtapTransport {
        let mut transport = MockCtapTransport::new();
        if cancelled {
            transport.expect_cancel().times(1).returning(|| {
                Box::pin(async {
                    Err(CtapTransportError::new(std::io::Error::other(
                        "cleanup failure",
                    )))
                })
            });
        } else {
            transport.expect_close().times(1).returning(|| {
                Box::pin(async {
                    Err(CtapTransportError::new(std::io::Error::other(
                        "cleanup failure",
                    )))
                })
            });
        }
        transport
    }

    keyringd_testkit::scenario! {
        group "OperationManager::cancel" {
            case "Coordinator cancellation reaches a candidate context" {
                async: tokio;
                timeout: std::time::Duration::from_secs(60);
                verify "propagate the coordinator cancellation token to its child context" {
                    let operation_manager = Arc::new(OperationManager::new(RecordingHandler::default()));
                    let entered = Arc::new(tokio::sync::Notify::new());
                    let running_manager = Arc::clone(&operation_manager);
                    let operation_entered = Arc::clone(&entered);
                    let operation = tokio::spawn(async move {
                        running_manager
                            .run_operation(
                                Instant::now() + Duration::from_secs(5),
                                |context| async move {
                                    let candidate = context.with_cancellation(context.cancellation.child_token());
                                    operation_entered.notify_one();
                                    candidate.cancellation.cancelled().await;
                                    Err::<(), _>(FidoClientError::Cancelled)
                                },
                            )
                            .await
                    });
                    entered.notified().await;
                    assert!(operation_manager.cancel(OperationId::new(1)));
                    let result = tokio::time::timeout(Duration::from_secs(1), operation)
                        .await
                        .expect("candidate must observe parent cancellation")
                        .unwrap();
                    assert_fido_error!(result, FidoClientError::Cancelled);
                    assert!(!operation_manager.cancel(OperationId::new(1)));
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "OperationManager::run_operation" {
            case "Dropping an operation releases the busy state" {
                async: tokio;
                timeout: std::time::Duration::from_secs(60);
                verify "allow a fresh operation after dropping the previous operation" {
                    let operation_manager = Arc::new(OperationManager::new(RecordingHandler::default()));
                    let active_manager = Arc::clone(&operation_manager);
                    let (started_sender, started) = tokio::sync::oneshot::channel();
                    let active = tokio::spawn(async move {
                        active_manager
                            .run_operation(
                                Instant::now() + Duration::from_secs(1),
                                |context| async move {
                                    started_sender.send(context.id).unwrap();
                                    std::future::pending::<Result<(), FidoClientError>>().await
                                },
                            )
                            .await
                    });
                    let operation_id = started.await.unwrap();
                    active.abort();
                    assert!(active.await.unwrap_err().is_cancelled());
                    assert!(!operation_manager.cancel(operation_id));
                    assert!(
                        operation_manager
                            .run_operation(Instant::now() + Duration::from_secs(1), |_| async {
                                Ok(())
                            })
                            .await
                            .is_ok()
                    );
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "AuthenticatorBroker::select_transport" {
            case "Unspecified and multiple hints use the handlers transport choice" {
                async: tokio;
                timeout: std::time::Duration::from_secs(60);
                verify "use the interaction handler to resolve absent or ambiguous transport hints" {
                    use keyringd_fido::webauthn::PublicKeyCredentialHint;
                    for hints in [
                        vec![],
                        vec![
                            PublicKeyCredentialHint::hybrid(),
                            PublicKeyCredentialHint::security_key(),
                        ],
                    ] {
                        for (transport, expected) in [
                            (AuthenticatorTransport::usb(), TransportSelection::Usb),
                            (AuthenticatorTransport::hybrid(), TransportSelection::Hybrid),
                        ] {
                            let handler = RecordingHandler {
                                transport_response: Some(transport),
                                ..RecordingHandler::default()
                            };
                            let operation = test_context(&handler, OperationId::new(60));
                            let selected = AuthenticatorBroker::select_transport(
                                TransportSelection::from_hints(&hints).unwrap(),
                                &operation,
                            )
                            .await
                            .unwrap();
                            assert_eq!(selected, expected);
                            assert_eq!(
                                handler.transport_requests.lock().unwrap()[0]
                                    .id()
                                    .operation_id(),
                                operation.id
                            );
                        }
                    }
                }
            }
        }
    }
    keyringd_testkit::scenario! {
        group "AuthenticatorBroker::select_transport" {
            case "Cancelling transport selection finishes without starting a transport" {
                async: tokio;
                timeout: std::time::Duration::from_secs(60);
                verify "return cancellation from the chooser without opening a backend connection" {
                    let handler = RecordingHandler::default();
                    let operation = test_context(&handler, OperationId::new(60));
                    assert_fido_error!(
                        AuthenticatorBroker::select_transport(None, &operation).await,
                        FidoClientError::Cancelled
                    );
                    assert!(handler.qr_uris.lock().unwrap().is_empty());
                }
            }
        }
    }
    keyringd_testkit::scenario! {
        group "AuthenticatorBroker::select_transport" {
            case "Transport selection wait obeys cancellation and deadline" {
                async: tokio;
                timeout: std::time::Duration::from_secs(60);
                verify "end a pending transport choice on token cancellation or deadline expiry" {
                    let handler = RecordingHandler {
                        transport_prompt_gate: Some(Arc::new(tokio::sync::Notify::new())),
                        ..RecordingHandler::default()
                    };
                    let mut operation = test_context(&handler, OperationId::new(60));
                    operation.cancellation.cancel();
                    assert_fido_error!(
                        AuthenticatorBroker::select_transport(None, &operation).await,
                        FidoClientError::Cancelled
                    );
                    operation.cancellation = tokio_util::sync::CancellationToken::new();
                    operation.deadline = Instant::now() + Duration::from_millis(10);
                    assert_fido_error!(
                        AuthenticatorBroker::select_transport(None, &operation).await,
                        FidoClientError::TimedOut
                    );
                }
            }
        }
    }

    fn cleanup_transport(cancelled: bool) -> MockCtapTransport {
        let mut transport = MockCtapTransport::new();
        transport
            .expect_cancel()
            .times(usize::from(cancelled))
            .returning(|| Box::pin(async { Ok(()) }));
        transport
            .expect_close()
            .times(usize::from(!cancelled))
            .returning(|| Box::pin(async { Ok(()) }));
        transport
    }

    fn tracked_hybrid_connector(
        entered: Arc<AtomicBool>,
        dropped: Arc<AtomicBool>,
    ) -> MockHybridChannelConnector {
        let mut connector = MockHybridChannelConnector::new();
        connector
            .expect_connect()
            .returning(move |_, cancellation| {
                let entered = entered.clone();
                let dropped = dropped.clone();
                Box::pin(async move {
                    entered.store(true, Ordering::SeqCst);
                    let _guard = PendingHybridGuard(dropped);
                    cancellation.cancelled().await;
                    Err(crate::HybridConnectionError::Cancelled)
                })
            });
        connector
    }
}
