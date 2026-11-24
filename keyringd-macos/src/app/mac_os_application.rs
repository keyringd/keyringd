use crate::app::*;

pub(crate) type MacOsApplication =
    keyringd_core::application::KeyringdApplication<MacOsDeviceKeyBackend, MacOsPasskeyBackend>;
