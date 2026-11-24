use super::super::PinPromptWindowDriver;
use crate::application::platform::linux::{ELEMENT_TIMEOUT, UiDriver};
use anyhow::Result;
use std::path::PathBuf;
use tokio::runtime::Runtime;
pub struct LinuxPinPromptWindow<'a> {
    pub(crate) runtime: &'a Runtime,
    pub(crate) session: UiDriver,
}
impl LinuxPinPromptWindow<'_> {
    async fn session(&self) -> Result<UiDriver> {
        let app = self.session.application();
        let dialog = app
            .wait_for_dialog("Enter your security key PIN", ELEMENT_TIMEOUT)
            .await?;
        Ok(app.within(&dialog))
    }
}
impl PinPromptWindowDriver for LinuxPinPromptWindow<'_> {
    fn screenshot(&self) -> Result<PathBuf> {
        self.runtime
            .block_on(async { self.session().await?.capture().await })
    }
    fn enter_pin(&self, pin: &str) -> Result<()> {
        self.runtime.block_on(async {
            let session = self.session().await?;
            let field = session
                .wait_for_password_entry("PIN", ELEMENT_TIMEOUT)
                .await?;
            session.replace_text(&field, pin).await
        })
    }
    fn submit(&self) -> Result<()> {
        self.runtime.block_on(async {
            let session = self.session().await?;
            let button = session
                .wait_for_enabled_action("Continue", ELEMENT_TIMEOUT)
                .await?;
            session.click(&button).await
        })
    }
    fn wait_closed(&self) -> Result<()> {
        self.runtime.block_on(async {
            self.session
                .application()
                .wait_for_visible_absence("Enter your security key PIN", ELEMENT_TIMEOUT)
                .await
        })
    }
}
