use windows::Win32::Foundation::{GlobalFree, HGLOBAL};
use windows::Win32::System::Memory::{GMEM_MOVEABLE, GlobalAlloc, GlobalLock, GlobalUnlock};

pub(super) struct GlobalUnicodeText {
    handle: Option<HGLOBAL>,
}

impl GlobalUnicodeText {
    pub(super) fn new(value: &str) -> std::io::Result<Self> {
        let utf16: Vec<u16> = value.encode_utf16().chain(Some(0)).collect();
        let byte_len = std::mem::size_of_val(utf16.as_slice());
        let memory =
            unsafe { GlobalAlloc(GMEM_MOVEABLE, byte_len) }.map_err(std::io::Error::other)?;
        let allocated = Self {
            handle: Some(memory),
        };

        let destination = unsafe { GlobalLock(memory) };
        if destination.is_null() {
            return Err(std::io::Error::last_os_error());
        }
        unsafe {
            std::ptr::copy_nonoverlapping(
                utf16.as_ptr().cast::<u8>(),
                destination.cast(),
                byte_len,
            );
            let _ = GlobalUnlock(memory);
        }
        Ok(allocated)
    }

    pub(super) fn handle(&self) -> HGLOBAL {
        self.handle.expect("the clipboard allocation should remain owned until ownership is transferred to Windows")
    }

    pub(super) fn transfer_to_clipboard(&mut self) {
        self.handle = None;
    }
}

impl Drop for GlobalUnicodeText {
    fn drop(&mut self) {
        if let Some(memory) = self.handle.take() {
            let _ = unsafe { GlobalFree(Some(memory)) };
        }
    }
}
