use keyringd_core::test_support::TestFidoMode;

/// Backend settings for one explicit application start.
#[derive(Clone, Default)]
pub struct ApplicationConfiguration {
    pub device_key_unavailable: bool,
    pub test_fido_mode: Option<TestFidoMode>,
    /// Enable the PXP BLE advertisement relay and apply the configured test tunnel route.
    #[cfg(any(
        target_os = "macos",
        windows,
        all(target_os = "linux", feature = "test-support")
    ))]
    pub accept_remote_hybrid_advertisements: bool,
    #[cfg(windows)]
    pub software_cng: bool,
}
