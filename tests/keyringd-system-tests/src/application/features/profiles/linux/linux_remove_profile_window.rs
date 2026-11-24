use crate::application::features::profiles::RemoveProfileWindowDriver;
use crate::application::platform::linux::{ELEMENT_TIMEOUT, UiDriver};
use anyhow::Result;
use std::path::PathBuf;
use tokio::runtime::Runtime;

pub struct LinuxRemoveProfileWindow<'a> {
    runtime: &'a Runtime,
    session: UiDriver,
    title: String,
}
impl<'a> LinuxRemoveProfileWindow<'a> {
    pub(super) async fn wait_for(
        runtime: &'a Runtime,
        session: UiDriver,
        name: &str,
    ) -> Result<Self> {
        let title = format!("Remove “{name}”?");
        let session = session.application();
        let dialog = session.wait_for_dialog(&title, ELEMENT_TIMEOUT).await?;
        Ok(Self {
            runtime,
            session: session.within(&dialog),
            title,
        })
    }
}
impl RemoveProfileWindowDriver for LinuxRemoveProfileWindow<'_> {
    fn screenshot(&self) -> Result<PathBuf> {
        self.runtime.block_on(self.session.capture())
    }
    fn confirm(&mut self) -> Result<()> {
        self.runtime.block_on(async {
            let (_, button) = self
                .session
                .wait_for_dialog_action("Remove", ELEMENT_TIMEOUT)
                .await?;
            self.session.click(&button).await?;
            self.session
                .application()
                .wait_for_visible_absence(&self.title, ELEMENT_TIMEOUT)
                .await
        })
    }
}
