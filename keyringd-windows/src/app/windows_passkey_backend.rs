use super::*;

#[cfg(not(feature = "test-support"))]
pub(crate) type WindowsPasskeyBackend = NativeWindowsPasskeyBackend;
