use super::TestPasskeyService;
use keyringd_core::service::PasskeyService;
use keyringd_fido::webauthn::PublicKeyCredentialCreationOptions;
use keyringd_fido_client::usb::UsbHidDeviceId;
use keyringd_fido_client::{
    ClientDataProvider, FidoClientError, GetAssertionsResult, MakeCredentialResult,
};
use std::{future::Future, sync::Arc};

mockall::mock! {
    /// A mock that sets the success and failure of FIDO operations for each test.
    pub(crate) PasskeyService {}
    impl PasskeyService for PasskeyService {
        fn make_credential(&self, options: PublicKeyCredentialCreationOptions, provider: Arc<dyn ClientDataProvider>)
            -> impl Future<Output = Result<MakeCredentialResult, FidoClientError>> + Send;
        fn get_assertions(&self, options: keyringd_fido::webauthn::PublicKeyCredentialRequestOptions, device_id: Option<UsbHidDeviceId>, provider: Arc<dyn ClientDataProvider>)
            -> impl Future<Output = Result<GetAssertionsResult, FidoClientError>> + Send;
    }
}

pub(crate) fn cancelled_fido() -> MockPasskeyService {
    let mut service = MockPasskeyService::new();
    service
        .expect_make_credential()
        .returning(|_, _| Box::pin(async { Err(FidoClientError::Cancelled) }));
    service
        .expect_get_assertions()
        .returning(|_, _, _| Box::pin(async { Err(FidoClientError::Cancelled) }));
    service
}

pub(crate) fn failing_assertion_fido() -> MockPasskeyService {
    let mut service = MockPasskeyService::new();
    let model = Arc::new(TestPasskeyService::new());
    service
        .expect_make_credential()
        .returning(move |options, provider| {
            let model = model.clone();
            Box::pin(async move { model.make_credential(options, provider).await })
        });
    service.expect_get_assertions().returning(|_, _, _| {
        Box::pin(async {
            Err(FidoClientError::Ctap(
                keyringd_fido::CtapStatusCode::NoCredentials,
            ))
        })
    });
    service
}
