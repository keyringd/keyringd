use super::*;

pub(crate) type WindowsApplication =
    keyringd_core::application::KeyringdApplication<WindowsDeviceKeyBackend, WindowsPasskeyBackend>;
