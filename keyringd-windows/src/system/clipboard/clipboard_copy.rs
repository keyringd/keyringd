use super::ClipboardSession;
use super::GlobalUnicodeText;

pub(crate) fn copy(value: &str) -> std::io::Result<()> {
    let clipboard = ClipboardSession::open()?;
    clipboard.clear()?;

    let mut text = GlobalUnicodeText::new(value)?;
    clipboard.set_unicode_text(&mut text)?;
    clipboard.close()
}
