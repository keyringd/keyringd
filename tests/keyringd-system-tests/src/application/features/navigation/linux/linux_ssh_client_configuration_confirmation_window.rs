use crate::application::platform::linux::{ELEMENT_TIMEOUT, UiDriver};
use anyhow::Result;
use std::{path::PathBuf, sync::Arc};
use tokio::runtime::Runtime;

pub struct LinuxSshClientConfigurationConfirmationWindow {
    runtime: Arc<Runtime>,
    session: UiDriver,
    parent: UiDriver,
    remove: bool,
}
impl LinuxSshClientConfigurationConfirmationWindow {
    pub(super) async fn wait_for(
        runtime: Arc<Runtime>,
        parent: UiDriver,
        remove: bool,
    ) -> Result<Self> {
        let application = parent.application();
        let dialog = application
            .wait_for_dialog(
                if remove {
                    "Remove SSH_AUTH_SOCK configuration?"
                } else {
                    "Configure SSH_AUTH_SOCK?"
                },
                ELEMENT_TIMEOUT,
            )
            .await?;
        let session = application.within(&dialog);
        Ok(Self {
            runtime,
            session,
            parent,
            remove,
        })
    }
    pub fn screenshot(&self) -> Result<PathBuf> {
        self.runtime.block_on(self.session.capture())
    }
    pub fn capture_screenshot(&self, name: &str) -> Result<()> {
        keyringd_testkit::record_image(name, self.screenshot()?)
    }
    pub fn confirm(&self) -> Result<()> {
        self.runtime.block_on(async {
            let button = self
                .session
                .wait_for_visible_action(
                    if self.remove {
                        "Unconfigure"
                    } else {
                        "Configure"
                    },
                    ELEMENT_TIMEOUT,
                )
                .await?;
            self.session.click(&button).await?;
            if self.remove {
                self.parent
                    .wait_for_visible_action("Configure", ELEMENT_TIMEOUT)
                    .await?;
            } else {
                self.parent
                    .wait_for_visible_name(
                        "Sign out and back in to apply the change.",
                        ELEMENT_TIMEOUT,
                    )
                    .await?;
            }
            Ok(())
        })
    }
}
