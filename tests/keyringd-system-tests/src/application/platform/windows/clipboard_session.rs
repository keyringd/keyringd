use anyhow::{Result, ensure};
use windows::Win32::Foundation::{E_ACCESSDENIED, HGLOBAL};
use windows::Win32::System::DataExchange::{CloseClipboard, GetClipboardData, OpenClipboard};
use windows::Win32::System::Memory::{GlobalLock, GlobalSize, GlobalUnlock};
use windows::Win32::System::Ole::CF_UNICODETEXT;

pub(crate) struct ClipboardSession;

impl ClipboardSession {
    pub(crate) fn try_open() -> Result<Option<Self>> {
        match unsafe { OpenClipboard(None) } {
            Ok(()) => Ok(Some(Self)),
            // Clipboard access is exclusive, including while the writer is closing it.
            Err(error) if error.code() == E_ACCESSDENIED => Ok(None),
            Err(error) => Err(error.into()),
        }
    }

    pub(crate) fn text(&self) -> Result<String> {
        // CF_UNICODETEXT is a null-terminated UTF-16 string in global memory.
        let handle = HGLOBAL(unsafe { GetClipboardData(CF_UNICODETEXT.0 as u32) }?.0);
        let bytes = unsafe { GlobalSize(handle) };
        ensure!(
            bytes >= 2 && bytes.is_multiple_of(2),
            "invalid clipboard text size"
        );
        let pointer = unsafe { GlobalLock(handle) };
        ensure!(!pointer.is_null(), "could not lock clipboard text");
        let result = (|| {
            let text = unsafe { std::slice::from_raw_parts(pointer.cast::<u16>(), bytes / 2) };
            let end = text
                .iter()
                .position(|unit| *unit == 0)
                .ok_or_else(|| anyhow::anyhow!("clipboard text is not null terminated"))?;
            Ok(String::from_utf16(&text[..end])?)
        })();
        unsafe {
            let _ = GlobalUnlock(handle);
        }
        result
    }
}

impl Drop for ClipboardSession {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseClipboard();
        }
    }
}
