mod secure_enclave_device_key_error;
mod secure_enclave_device_key_service;
mod security_framework_error;

#[cfg(feature = "test-support")]
pub(crate) use secure_enclave_device_key_error::SecureEnclaveDeviceKeyError;
pub(crate) use secure_enclave_device_key_service::SecureEnclaveDeviceKeyService as MacOsDeviceKeyService;
mod authentication_context;
mod key_local_authentication;
