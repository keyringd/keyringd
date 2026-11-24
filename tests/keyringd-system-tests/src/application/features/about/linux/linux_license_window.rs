use super::super::{LicenseDocumentDriver, LicenseWindowDriver};
use crate::application::platform::linux::{ELEMENT_TIMEOUT, UiDriver};
use anyhow::Result;
use std::path::PathBuf;
use tokio::runtime::Runtime;
pub struct LinuxLicenseWindow<'a> {
    pub(super) runtime: &'a Runtime,
    pub(super) session: UiDriver,
    pub(super) document: keyringd_ui_core::about_window::LicenseDocument,
}
impl LinuxLicenseWindow<'_> {
    pub fn scroll_to_bottom(&self) -> Result<()> {
        self.runtime.block_on(async {
            let label = self
                .session
                .wait_for_visible_name("License text", ELEMENT_TIMEOUT)
                .await?;
            let window = format!("^{}$", self.document.title());
            self.session.focus_with_tab(&label, &window).await?;
            self.session
                .press_key_sequence(&window, &["ctrl+End"])
                .await
        })
    }
}
impl LicenseDocumentDriver for LinuxLicenseWindow<'_> {
    fn text(&self) -> Result<String> {
        self.runtime.block_on(async {
            let label = self.session.named("License text");
            label.text(ELEMENT_TIMEOUT).await
        })
    }
    fn screenshot(&self) -> Result<PathBuf> {
        self.runtime.block_on(self.session.capture())
    }
}
impl LicenseWindowDriver for LinuxLicenseWindow<'_> {
    fn close(&self) -> Result<()> {
        self.runtime.block_on(async {
            self.session
                .press_key_sequence(&format!("^{}$", self.document.title()), &["Escape"])
                .await?;
            self.session
                .application()
                .wait_for_visible_absence(self.document.title(), ELEMENT_TIMEOUT)
                .await
        })
    }
}
