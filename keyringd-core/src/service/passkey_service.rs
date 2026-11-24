use keyringd_fido::webauthn::{
    PublicKeyCredentialCreationOptions, PublicKeyCredentialRequestOptions,
};
use keyringd_fido_client::{
    ClientDataProvider, FidoClientError, GetAssertionsResult, MakeCredentialResult,
};
use std::future::Future;

/// Perform external Passkey registration/authentication according to the Options Hints.
/// For subsequent authentication, you can use only the same key by specifying the USB device ID included in the previous result.
pub trait PasskeyService: Send + Sync {
    fn make_credential(
        &self,
        options: PublicKeyCredentialCreationOptions,
        client_data_provider: std::sync::Arc<dyn ClientDataProvider>,
    ) -> impl Future<Output = Result<MakeCredentialResult, FidoClientError>> + Send;
    fn get_assertions(
        &self,
        options: PublicKeyCredentialRequestOptions,
        usb_device_id: Option<keyringd_fido_client::usb::UsbHidDeviceId>,
        client_data_provider: std::sync::Arc<dyn ClientDataProvider>,
    ) -> impl Future<Output = Result<GetAssertionsResult, FidoClientError>> + Send;
}

impl<T: PasskeyService> PasskeyService for std::sync::Arc<T> {
    async fn make_credential(
        &self,
        options: PublicKeyCredentialCreationOptions,
        client_data_provider: std::sync::Arc<dyn ClientDataProvider>,
    ) -> Result<MakeCredentialResult, FidoClientError> {
        (**self)
            .make_credential(options, client_data_provider)
            .await
    }
    async fn get_assertions(
        &self,
        options: PublicKeyCredentialRequestOptions,
        usb_device_id: Option<keyringd_fido_client::usb::UsbHidDeviceId>,
        client_data_provider: std::sync::Arc<dyn ClientDataProvider>,
    ) -> Result<GetAssertionsResult, FidoClientError> {
        (**self)
            .get_assertions(options, usb_device_id, client_data_provider)
            .await
    }
}
