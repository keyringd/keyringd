#[cfg(any(test, feature = "test-support"))]
use windows::Win32::Security::Cryptography::MS_KEY_STORAGE_PROVIDER;
use windows::Win32::Security::Cryptography::MS_PLATFORM_CRYPTO_PROVIDER;
use windows::core::PCWSTR;

#[derive(Clone, Copy)]
pub(super) enum CngProvider {
    Platform,
    #[cfg(any(test, feature = "test-support"))]
    Software,
}

impl CngProvider {
    pub(super) fn name(self) -> PCWSTR {
        match self {
            Self::Platform => MS_PLATFORM_CRYPTO_PROVIDER,
            #[cfg(any(test, feature = "test-support"))]
            Self::Software => MS_KEY_STORAGE_PROVIDER,
        }
    }
}
