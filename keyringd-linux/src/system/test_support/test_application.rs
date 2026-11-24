use crate::system::device_key::EsapiDeviceKeyService;
use keyringd_core::application::KeyringdApplication;
use keyringd_core::test_support::TestPasskeyService;
pub(crate) type TestApplication = KeyringdApplication<EsapiDeviceKeyService, TestPasskeyService>;
