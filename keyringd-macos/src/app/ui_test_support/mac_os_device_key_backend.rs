use keyringd_core::model::device_key::DevicePrivateKeyHandle;
use keyringd_core::service::DeviceKeyGenerationRequest;
use keyringd_core::service::DeviceKeyMaterial;
use keyringd_core::service::DeviceKeyProtectionChangeRequest;
use keyringd_core::service::DeviceKeyProtectionChangeResult;
use keyringd_core::service::DeviceKeyService;
use keyringd_core::service::DeviceKeySigningRequest;
use keyringd_core::test_support::SoftwareDeviceKey;

use crate::system::device_key::MacOsDeviceKeyService;
/// The composition boundary UI tests substitute at. The default fixture keeps
/// the Secure Enclave and the user's keychain out of the run entirely.
pub(crate) enum MacOsDeviceKeyBackend {
    Native(MacOsDeviceKeyService),
    Test(SoftwareDeviceKey),
}

use crate::app::ui_test_support::DeviceKeyBackendError;

impl From<MacOsDeviceKeyService> for MacOsDeviceKeyBackend {
    fn from(service: MacOsDeviceKeyService) -> Self {
        Self::Native(service)
    }
}

impl DeviceKeyService for MacOsDeviceKeyBackend {
    type Error = DeviceKeyBackendError;
    type OperationContext = ();

    async fn check_availability(&self) -> Result<(), Self::Error> {
        match self {
            Self::Native(service) => Ok(service.check_availability().await?),
            Self::Test(service) => Ok(service.check_availability().await?),
        }
    }

    async fn generate_key(
        &self,
        context: &(),
        request: DeviceKeyGenerationRequest<'_>,
    ) -> Result<DeviceKeyMaterial, Self::Error> {
        match self {
            Self::Native(service) => Ok(service.generate_key(context, request).await?),
            Self::Test(service) => Ok(service.generate_key(context, request).await?),
        }
    }

    async fn change_key_protection(
        &self,
        context: &(),
        request: DeviceKeyProtectionChangeRequest<'_>,
    ) -> Result<DeviceKeyProtectionChangeResult, Self::Error> {
        match self {
            Self::Native(service) => Ok(service.change_key_protection(context, request).await?),
            Self::Test(service) => Ok(service.change_key_protection(context, request).await?),
        }
    }

    async fn sign(
        &self,
        context: &(),
        request: DeviceKeySigningRequest<'_>,
    ) -> Result<keyringd_core::service::DeviceKeySignature, Self::Error> {
        match self {
            Self::Native(service) => Ok(service.sign(context, request).await?),
            Self::Test(service) => Ok(service.sign(context, request).await?),
        }
    }

    async fn delete_key(
        &self,
        context: &(),
        handle: &DevicePrivateKeyHandle,
    ) -> Result<(), Self::Error> {
        match self {
            Self::Native(service) => Ok(service.delete_key(context, handle).await?),
            Self::Test(service) => Ok(service.delete_key(context, handle).await?),
        }
    }

    fn is_user_cancellation(&self, error: &Self::Error) -> bool {
        match (self, error) {
            (Self::Native(service), DeviceKeyBackendError::Native(error)) => {
                service.is_user_cancellation(error)
            }
            (Self::Test(service), DeviceKeyBackendError::Test(error)) => {
                service.is_user_cancellation(error)
            }
            _ => false,
        }
    }
}
