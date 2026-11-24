use anyhow::{Result, ensure};
use windows::Win32::UI::HiDpi::{
    DPI_AWARENESS_CONTEXT, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, SetThreadDpiAwarenessContext,
};

pub(crate) struct DpiAwarenessGuard(DPI_AWARENESS_CONTEXT);

impl DpiAwarenessGuard {
    pub(crate) fn enter() -> Result<Self> {
        let previous =
            unsafe { SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) };
        ensure!(
            !previous.0.is_null(),
            "could not set screenshot DPI awareness"
        );
        Ok(Self(previous))
    }
}

impl Drop for DpiAwarenessGuard {
    fn drop(&mut self) {
        unsafe {
            SetThreadDpiAwarenessContext(self.0);
        }
    }
}
