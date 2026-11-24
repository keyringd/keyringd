use super::LinuxSshClientConfigurationConfirmationWindow;
use crate::application::platform::linux::{ELEMENT_TIMEOUT, UiDriver};
use anyhow::Result;
use std::{path::PathBuf, sync::Arc};
use tokio::runtime::Runtime;

pub struct LinuxSshClientConfigurationWindow {
    runtime: Arc<Runtime>,
    session: UiDriver,
}
impl LinuxSshClientConfigurationWindow {
    pub(crate) fn open(runtime: Arc<Runtime>, session: UiDriver) -> Result<Self> {
        let session = runtime.block_on(async {
            let session = session.application();
            let action = session
                .wait_for_enabled_action("Configure SSH Clients…", ELEMENT_TIMEOUT)
                .await?;
            session.click(&action).await?;
            let dialog = session
                .wait_for_dialog("Configure SSH Clients to Use Keyringd", ELEMENT_TIMEOUT)
                .await?;
            Ok::<_, anyhow::Error>(session.within(&dialog))
        })?;
        Ok(Self { runtime, session })
    }
    pub fn screenshot(&self) -> Result<PathBuf> {
        self.runtime.block_on(self.session.capture())
    }
    pub fn capture_screenshot(&self, name: &str) -> Result<()> {
        keyringd_testkit::record_image(name, self.screenshot()?)
    }
    pub fn open_confirmation(&self) -> Result<LinuxSshClientConfigurationConfirmationWindow> {
        self.open_confirmation_for(false)
    }
    pub fn open_removal_confirmation(
        &self,
    ) -> Result<LinuxSshClientConfigurationConfirmationWindow> {
        self.open_confirmation_for(true)
    }
    fn open_confirmation_for(
        &self,
        remove: bool,
    ) -> Result<LinuxSshClientConfigurationConfirmationWindow> {
        self.runtime.block_on(async {
            let button = self
                .session
                .wait_for_visible_action(
                    if remove { "Unconfigure" } else { "Configure" },
                    ELEMENT_TIMEOUT,
                )
                .await?;
            self.session.click(&button).await?;
            LinuxSshClientConfigurationConfirmationWindow::wait_for(
                Arc::clone(&self.runtime),
                self.session.clone(),
                remove,
            )
            .await
        })
    }
}
