use futures_util::future::BoxFuture;
use keyringd_fido::{AuthenticatorTransport, ctap::Pin};
use keyringd_fido_client::{
    FidoInteractionHandler, FidoInteractionHandlerError, InteractionResponse, NewPinInteraction,
    OperationStatus, PinInteraction, TransportInteraction,
};

mockall::mock! {
    /// A mock that sets the route/PIN interaction and progress notifications for each test.
    pub(crate) FidoInteractionHandler {}
    impl FidoInteractionHandler for FidoInteractionHandler {
        fn request_transport(&self, request: TransportInteraction)
            -> BoxFuture<'static, Result<InteractionResponse<Option<AuthenticatorTransport>>, FidoInteractionHandlerError>>;
        fn on_operation_status_changed(&self, event: OperationStatus)
            -> BoxFuture<'static, Result<(), FidoInteractionHandlerError>>;
        fn request_pin(&self, request: PinInteraction)
            -> BoxFuture<'static, Result<InteractionResponse<Option<Pin>>, FidoInteractionHandlerError>>;
        fn request_new_pin(&self, request: NewPinInteraction)
            -> BoxFuture<'static, Result<InteractionResponse<Option<Pin>>, FidoInteractionHandlerError>>;
    }
}

pub(crate) fn usb_only_handler() -> MockFidoInteractionHandler {
    let mut handler = MockFidoInteractionHandler::new();
    handler.expect_request_transport().returning(|request| {
        Box::pin(async move {
            Ok(InteractionResponse::new(
                request.id(),
                Some(AuthenticatorTransport::usb()),
            ))
        })
    });
    handler
        .expect_on_operation_status_changed()
        .returning(|_| Box::pin(async { Ok(()) }));
    handler.expect_request_pin().never();
    handler.expect_request_new_pin().never();
    handler
}
