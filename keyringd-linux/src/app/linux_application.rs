use keyringd_ui_core::key_operation::InteractivePasskeyService;

pub(crate) type LinuxApplication = keyringd_core::application::KeyringdApplication<
    crate::system::device_key::EsapiDeviceKeyService,
    std::sync::Arc<InteractivePasskeyService>,
>;
