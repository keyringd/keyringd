mod esapi_device_key_service;
mod esapi_error;
mod key_handle_data;
mod key_handle_error;
#[cfg(all(test, feature = "test-support"))]
pub(crate) mod test_support;
pub(crate) use esapi_device_key_service::EsapiDeviceKeyService;
pub(crate) use esapi_error::EsapiError;
mod tpm_access;
use key_handle_error::KeyHandleError;
pub(crate) use tpm_access::{can_grant_tpm_access, grant_tpm_access};
