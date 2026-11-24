use crate::model::device_key::DevicePrivateKeyHandle;
use crate::service::DeviceKeyGenerationRequest;
use crate::service::DeviceKeyMaterial;
use crate::service::DeviceKeyProtectionChangeRequest;
use crate::service::DeviceKeyProtectionChangeResult;
use crate::service::DeviceKeySigningRequest;

/// Storage and signing operations for the device key.
///
/// The device key is a non-exportable key bound to the PC's
/// hardware (TPM or Secure Enclave) that authenticates SSH sessions.
pub trait DeviceKeyService: Send + Sync {
    type Error: std::error::Error + Send + Sync + 'static;
    type OperationContext: Send + Sync;

    /// Check whether hardware-backed Device Key storage can be used.
    /// Success does not guarantee a later key operation will succeed.
    fn check_availability(
        &self,
    ) -> impl std::future::Future<Output = Result<(), Self::Error>> + Send;

    /// Generate a key pair and report its handle, public key, and persistence.
    fn generate_key(
        &self,
        context: &Self::OperationContext,
        request: DeviceKeyGenerationRequest<'_>,
    ) -> impl std::future::Future<Output = Result<DeviceKeyMaterial, Self::Error>> + Send;

    /// Change the key's local user-verification policy. Implementations should
    /// preserve the public key when the platform supports doing so.
    /// The result describes the updated key and the unused resources to delete
    /// after a successful commit or after a failure. Cleanup is independent of
    /// whether the public key changes.
    fn change_key_protection(
        &self,
        context: &Self::OperationContext,
        request: DeviceKeyProtectionChangeRequest<'_>,
    ) -> impl std::future::Future<Output = Result<DeviceKeyProtectionChangeResult, Self::Error>> + Send;

    /// Open the selected key, derive its actual local-authentication requirement,
    /// and invoke the digest callback for that same signing operation.
    /// Return a signature only after signing and required authentication succeed.
    /// Never derive authentication requirements from Profile preferences.
    fn sign(
        &self,
        context: &Self::OperationContext,
        request: DeviceKeySigningRequest<'_>,
    ) -> impl std::future::Future<Output = Result<crate::service::DeviceKeySignature, Self::Error>> + Send;

    fn delete_key(
        &self,
        context: &Self::OperationContext,
        handle: &DevicePrivateKeyHandle,
    ) -> impl std::future::Future<Output = Result<(), Self::Error>> + Send;

    /// Returns whether an error represents an explicit cancellation by the user.
    fn is_user_cancellation(&self, _error: &Self::Error) -> bool {
        false
    }
}
