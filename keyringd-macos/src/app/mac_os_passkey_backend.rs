use crate::app::*;

#[cfg(not(feature = "test-support"))]
pub(crate) type MacOsPasskeyBackend = NativeMacOsPasskeyBackend;
