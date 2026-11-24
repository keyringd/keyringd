use windows::Win32::Foundation::HANDLE;
use windows::Win32::System::DataExchange::{
    CloseClipboard, EmptyClipboard, OpenClipboard, SetClipboardData,
};

use super::GlobalUnicodeText;

pub(super) struct ClipboardSession {
    open: bool,
}

impl ClipboardSession {
    pub(super) fn open() -> std::io::Result<Self> {
        unsafe { OpenClipboard(None) }.map_err(std::io::Error::other)?;
        Ok(Self { open: true })
    }

    pub(super) fn clear(&self) -> std::io::Result<()> {
        unsafe { EmptyClipboard() }.map_err(std::io::Error::other)
    }

    pub(super) fn set_unicode_text(&self, text: &mut GlobalUnicodeText) -> std::io::Result<()> {
        const CF_UNICODETEXT: u32 = 13;
        unsafe { SetClipboardData(CF_UNICODETEXT, Some(HANDLE(text.handle().0))) }
            .map_err(std::io::Error::other)?;
        text.transfer_to_clipboard();
        Ok(())
    }

    pub(super) fn close(mut self) -> std::io::Result<()> {
        unsafe { CloseClipboard() }.map_err(std::io::Error::other)?;
        self.open = false;
        Ok(())
    }
}

impl Drop for ClipboardSession {
    fn drop(&mut self) {
        if self.open {
            let _ = unsafe { CloseClipboard() };
        }
    }
}
