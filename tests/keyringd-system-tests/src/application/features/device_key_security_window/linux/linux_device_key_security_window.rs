use crate::application::platform::linux::{ELEMENT_TIMEOUT, UiDriver};
use anyhow::Result;
use std::path::PathBuf;
use tokio::runtime::Runtime;
/// The standalone Password dialog shown while signing with a protected Device Key.
pub struct LinuxDeviceKeySecurityWindow<'a> {
    pub(crate) runtime: &'a Runtime,
    pub(crate) session: UiDriver,
}
impl LinuxDeviceKeySecurityWindow<'_> {
    async fn session(&self) -> Result<UiDriver> {
        let app = self.session.application();
        let dialog = app
            .wait_for_dialog("Unlock Device Key", ELEMENT_TIMEOUT)
            .await?;
        Ok(app.within(&dialog))
    }
    pub fn wait_open(&self) -> Result<()> {
        self.runtime.block_on(self.session())?;
        Ok(())
    }
    pub fn screenshot(&self) -> Result<PathBuf> {
        self.runtime
            .block_on(async { self.session().await?.capture().await })
    }
    pub fn capture_screenshot(&self, name: &str) -> Result<()> {
        keyringd_testkit::record_image(name, self.screenshot()?)
    }
    pub fn unlock(&self, password: &str) -> Result<()> {
        self.runtime.block_on(async {
            let session = self.session().await?;
            let entry = session
                .wait_for_password_entry("Password", ELEMENT_TIMEOUT)
                .await?;
            session.replace_text(&entry, password).await?;
            let button = session
                .wait_for_dialog_action("Continue", ELEMENT_TIMEOUT)
                .await?
                .1;
            session.click(&button).await
        })
    }
}
