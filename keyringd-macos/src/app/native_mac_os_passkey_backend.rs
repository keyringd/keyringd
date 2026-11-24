use keyringd_ui_core::key_operation::InteractivePasskeyService;

/// The product FIDO service. UI tests swap this out at the composition
/// boundary; everything below it stays the real implementation.
pub(crate) type NativeMacOsPasskeyBackend = std::sync::Arc<InteractivePasskeyService>;
