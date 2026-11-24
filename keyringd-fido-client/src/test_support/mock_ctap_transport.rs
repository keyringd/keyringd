use crate::{CtapTransport, CtapTransportError};
use futures_util::future::BoxFuture;
use keyringd_fido::{CtapRequest, CtapResponse};

// Mock's Future does not hold the borrowed request and returns the owned response/wait state.
// You can keep it at the expected value of Mockall by narrowing down the return value to static.
mockall::mock! {
    /// A common mock that specifies the request/response and number of calls for CTAP communication for each test.
    pub(crate) CtapTransport {}
    impl CtapTransport for CtapTransport {
        fn transmit<'a>(&'a mut self, request: &'a CtapRequest)
            -> BoxFuture<'static, Result<CtapResponse, CtapTransportError>>;
        fn cancel(&mut self) -> BoxFuture<'static, Result<(), CtapTransportError>>;
        fn close(&mut self) -> BoxFuture<'static, Result<(), CtapTransportError>>;
    }
}

/// The authenticator model for key exchange, signature, and credentials is used as the source of the response of the mock.
pub(crate) fn model_transport(
    authenticator: std::sync::Arc<keyringd_test_authenticator::ModelCtapAuthenticator>,
) -> MockCtapTransport {
    let mut transport = MockCtapTransport::new();
    let model = authenticator.clone();
    transport.expect_transmit().returning(move |request| {
        let response = model
            .transmit(request.command(), request.payload())
            .map_err(CtapTransportError::new)
            .and_then(|encoded| {
                CtapResponse::from_bytes(&encoded).map_err(CtapTransportError::new)
            });
        Box::pin(async move { response })
    });
    transport.expect_cancel().returning(move || {
        authenticator.cancel();
        Box::pin(async { Ok(()) })
    });
    transport
        .expect_close()
        .returning(|| Box::pin(async { Ok(()) }));
    transport
}
