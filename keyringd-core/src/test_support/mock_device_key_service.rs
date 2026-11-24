use super::{MockDeviceKeyServiceError, test_device_public_key};
use keyringd_core::model::device_key::DevicePrivateKeyHandle;
use keyringd_core::service::DeviceKeyGenerationRequest;
use keyringd_core::service::DeviceKeyMaterial;
use keyringd_core::service::DeviceKeyProtectionChangeRequest;
use keyringd_core::service::DeviceKeyProtectionChangeResult;
use keyringd_core::service::DeviceKeyService;
use keyringd_core::service::DeviceKeySigningRequest;

mockall::mock! {
    /// A mock that sets the request and results of the Device Key operation on a per-test basis.
    pub(crate) DeviceKeyService {}
    impl DeviceKeyService for DeviceKeyService {
        type Error = MockDeviceKeyServiceError;
        type OperationContext = ();
        async fn check_availability(&self) -> Result<(), MockDeviceKeyServiceError>;
        async fn generate_key<'a>(&self, context: &(), request: DeviceKeyGenerationRequest<'a>)
            -> Result<DeviceKeyMaterial, MockDeviceKeyServiceError>;
        async fn change_key_protection<'a>(&self, context: &(), request: DeviceKeyProtectionChangeRequest<'a>)
            -> Result<DeviceKeyProtectionChangeResult, MockDeviceKeyServiceError>;
        async fn sign<'a>(&self, context: &(), request: DeviceKeySigningRequest<'a>)
            -> Result<keyringd_core::service::DeviceKeySignature, MockDeviceKeyServiceError>;
        async fn delete_key(&self, context: &(), handle: &DevicePrivateKeyHandle)
            -> Result<(), MockDeviceKeyServiceError>;
    }
}

/// A general-purpose success response used by the Agent test. A test that limits the calls sets the expected value from new().
pub(crate) fn successful_device_keys() -> MockDeviceKeyService {
    let mut service = MockDeviceKeyService::new();
    service.expect_generate_key().returning(|_, request| {
        Ok(DeviceKeyMaterial {
            handle: DevicePrivateKeyHandle::new(b"mock-key-handle".to_vec()),
            public_key: test_device_public_key(request.algorithm),
        })
    });
    service
        .expect_change_key_protection()
        .returning(|_, request| {
            Ok(DeviceKeyProtectionChangeResult::updated(
                DeviceKeyMaterial {
                    handle: request.handle.clone(),
                    public_key: test_device_public_key(request.algorithm),
                },
            ))
        });
    service.expect_sign().returning(|_, _| {
        let mut bytes = [0u8; 64];
        bytes[..32].fill(1);
        bytes[32..].fill(2);
        Ok(keyringd_core::service::DeviceKeySignature {
            signature: bytes.to_vec(),
            local_authentication:
                keyringd_core::model::device_key::LocalAuthentication::NotRequired,
        })
    });
    service.expect_delete_key().returning(|_, _| Ok(()));
    service
}
