use windows::Win32::Security::Cryptography::{
    NCRYPT_HANDLE, NCRYPT_PROV_HANDLE, NCryptFreeObject, NCryptOpenStorageProvider,
};

use super::CngError;
use super::CngProvider;

pub(super) struct ProviderHandle(pub(super) NCRYPT_PROV_HANDLE);

impl ProviderHandle {
    pub(super) fn open(provider: CngProvider) -> Result<Self, CngError> {
        let mut handle = NCRYPT_PROV_HANDLE::default();
        unsafe {
            NCryptOpenStorageProvider(&mut handle, provider.name(), 0)?;
        }
        Ok(Self(handle))
    }
}

impl Drop for ProviderHandle {
    fn drop(&mut self) {
        let _ = unsafe { NCryptFreeObject(NCRYPT_HANDLE(self.0.0)) };
    }
}
