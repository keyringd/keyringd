use super::super::QrPromptWindowDriver;
use crate::application::platform::linux::{ELEMENT_TIMEOUT, KEY_OPERATION_TIMEOUT, UiDriver};
use anyhow::Result;
use std::path::PathBuf;
use tokio::runtime::Runtime;
pub struct LinuxQrPromptWindow<'a> {
    pub(crate) runtime: &'a Runtime,
    pub(crate) session: UiDriver,
}
impl LinuxQrPromptWindow<'_> {
    async fn view(&self) -> Result<UiDriver> {
        self.session
            .application()
            .wait_for_group("Authenticator transport choices", KEY_OPERATION_TIMEOUT)
            .await
    }
}
impl QrPromptWindowDriver for LinuxQrPromptWindow<'_> {
    fn read_hybrid_qr(&self) -> Result<(String, PathBuf)> {
        self.runtime
            .block_on(async { self.view().await?.read_hybrid_qr(None).await })
    }
    fn security_key_enabled(&self) -> Result<bool> {
        self.runtime.block_on(async {
            let session = self.view().await?;
            let button = session.named("Use security key…").action();
            button.enabled(ELEMENT_TIMEOUT).await
        })
    }
    fn press_use_security_key(&self) -> Result<()> {
        self.runtime.block_on(async {
            let session = self.view().await?;
            let button = session
                .wait_for_enabled_action("Use security key…", ELEMENT_TIMEOUT)
                .await?;
            session.click(&button).await
        })
    }
    fn screenshot(&self) -> Result<PathBuf> {
        self.runtime
            .block_on(async { self.view().await?.capture().await })
    }
    fn wait_closed(&self) -> Result<()> {
        self.runtime.block_on(async {
            self.session
                .application()
                .wait_for_visible_absence("Authenticator transport choices", ELEMENT_TIMEOUT)
                .await
        })
    }
}
