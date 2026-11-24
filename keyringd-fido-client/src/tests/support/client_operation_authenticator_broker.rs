use crate::FidoClientError;
use crate::client::operation::OperationId;
use crate::client::operation::test_support::BrokerTestClient;
use crate::client::operation::test_support::ModelHybridConnector;
use crate::client::operation::test_support::RecordingHandler;
use crate::client::operation::test_support::make_options;
use crate::test_support::MockCtapTransport;
use crate::test_support::assert_fido_error;
use keyringd_fido::UserVerificationRequirement;
use keyringd_fido::ctap::AuthenticatorGetInfoResponse;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::time::Duration;
use std::time::Instant;

pub(crate) fn assert_protocol_error<E>(error: &FidoClientError, predicate: impl FnOnce(&E) -> bool)
where
    E: std::error::Error + 'static,
{
    assert!(matches!(&error, FidoClientError::Protocol(_)));
    let source = std::error::Error::source(error).expect("protocol error source");
    assert!(predicate(
        source.downcast_ref::<E>().expect("protocol error type")
    ));
}

pub(crate) async fn run_interrupted_hybrid_ctap_operation(cancel_explicitly: bool) {
    let authenticator = Arc::new(keyringd_test_authenticator::ModelCtapAuthenticator::new());
    let closed_channels = Arc::new(AtomicUsize::new(0));
    let shutdown_frames = Arc::new(AtomicUsize::new(0));
    let ctap_entered = Arc::new(AtomicBool::new(false));
    let connector = Arc::new(ModelHybridConnector {
        authenticator,
        closed_channels: Arc::clone(&closed_channels),
        shutdown_frames: Arc::clone(&shutdown_frames),
        ctap_entered: Some(Arc::clone(&ctap_entered)),
        resume_ctap: Some(Arc::new(tokio::sync::Notify::new())),
    });
    let handler = Arc::new(RecordingHandler::default());
    let service = Arc::new(BrokerTestClient::with_hybrid_connector(
        Arc::clone(&handler),
        connector,
    ));
    let options = make_options(UserVerificationRequirement::Discouraged, 1);
    let deadline = Instant::now()
        + if cancel_explicitly {
            Duration::from_secs(1)
        } else {
            Duration::from_millis(20)
        };
    let operation_service = Arc::clone(&service);
    let operation = tokio::spawn(async move {
        operation_service
            .make_credential_with_deadline(options, deadline)
            .await
    });
    while !ctap_entered.load(Ordering::SeqCst) {
        tokio::task::yield_now().await;
    }
    if cancel_explicitly {
        assert!(service.cancel(OperationId::new(1)));
    }

    let result = operation.await.unwrap();
    if cancel_explicitly {
        assert_fido_error!(result, FidoClientError::Cancelled);
    } else {
        assert_fido_error!(result, FidoClientError::TimedOut);
    }
    assert_eq!(shutdown_frames.load(Ordering::SeqCst), 1);
    assert_eq!(closed_channels.load(Ordering::SeqCst), 1);
    assert_eq!(
        handler
            .events
            .lock()
            .unwrap()
            .iter()
            .filter(|event| **event == "backend_finished")
            .count(),
        1
    );
}

pub(crate) fn fixed_get_info_transport(info: AuthenticatorGetInfoResponse) -> MockCtapTransport {
    let mut transport = MockCtapTransport::new();
    transport
        .expect_transmit()
        .times(1)
        .withf(|request| {
            request.command() == keyringd_fido::CtapCommand::GetInfo && request.payload().is_empty()
        })
        .return_once(move |_| {
            let response = keyringd_fido::CtapResponse::new(
                keyringd_fido::CtapStatusCode::Ok,
                info.to_cbor_bytes().unwrap(),
            );
            Box::pin(async move { Ok(response) })
        });
    transport
        .expect_close()
        .returning(|| Box::pin(async { Ok(()) }));
    transport
}
