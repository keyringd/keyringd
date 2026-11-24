use keyringd_fido::webauthn::PublicKeyCredentialCreationOptions;
use keyringd_fido_client::{
    ClientDataProvider, FidoClientError, GetAssertionsResult, MakeCredentialResult,
};
use std::sync::Arc;

use keyringd_core::service::PasskeyService;

use crate::app::NativeMacOsPasskeyBackend;
use keyringd_core::test_support::TestPasskeyService;

pub(crate) enum MacOsPasskeyBackend {
    Native(NativeMacOsPasskeyBackend),
    Test(
        Arc<TestPasskeyService>,
        Arc<keyringd_ui_core::key_operation::KeyOperationCoordinator>,
    ),
}
impl From<NativeMacOsPasskeyBackend> for MacOsPasskeyBackend {
    fn from(service: NativeMacOsPasskeyBackend) -> Self {
        Self::Native(service)
    }
}
impl PasskeyService for MacOsPasskeyBackend {
    async fn make_credential(
        &self,
        options: PublicKeyCredentialCreationOptions,
        client_data_provider: std::sync::Arc<dyn ClientDataProvider>,
    ) -> Result<MakeCredentialResult, FidoClientError> {
        match self {
            Self::Native(service) => service.make_credential(options, client_data_provider).await,
            Self::Test(service, handler) => {
                keyringd_ui_core::key_operation::InteractivePasskeyService::run(
                    handler,
                    options.hints(),
                    options.timeout(),
                    |_, _| service.make_credential(options.clone(), client_data_provider.clone()),
                    |id| {
                        service.cancel(id);
                    },
                )
                .await
            }
        }
    }
    async fn get_assertions(
        &self,
        options: keyringd_fido::webauthn::PublicKeyCredentialRequestOptions,
        usb_device_id: Option<keyringd_fido_client::usb::UsbHidDeviceId>,
        provider: std::sync::Arc<dyn ClientDataProvider>,
    ) -> Result<GetAssertionsResult, FidoClientError> {
        match self {
            Self::Native(service) => {
                service
                    .get_assertions(options, usb_device_id, provider)
                    .await
            }
            Self::Test(service, handler) => {
                let client_data = provider
                    .provide(options.challenge())
                    .map_err(|error| FidoClientError::Protocol(Box::new(error)))?;
                let provider: std::sync::Arc<dyn ClientDataProvider> =
                    std::sync::Arc::new(client_data);
                keyringd_ui_core::key_operation::InteractivePasskeyService::run(
                    handler,
                    options.hints(),
                    options.timeout(),
                    |_, timeout| {
                        service.get_assertions(
                            options.clone().with_timeout(timeout),
                            usb_device_id.clone(),
                            provider.clone(),
                        )
                    },
                    |id| {
                        service.cancel(id);
                    },
                )
                .await
            }
        }
    }
}
