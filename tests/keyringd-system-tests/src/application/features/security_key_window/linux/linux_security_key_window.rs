use super::super::SecurityKeyWindowDriver;
use crate::application::platform::linux::atspi::is_removed_accessible;
use crate::application::platform::linux::{ELEMENT_TIMEOUT, KEY_OPERATION_TIMEOUT, UiDriver};
use anyhow::{Context, Result};
use std::path::PathBuf;
use tokio::runtime::Runtime;
pub struct LinuxSecurityKeyWindow<'a> {
    pub(crate) runtime: &'a Runtime,
    pub(crate) session: UiDriver,
}
impl LinuxSecurityKeyWindow<'_> {
    const HEADINGS: [&str; 3] = [
        "Choose your security key",
        "Approve on your security key",
        "Verify your identity on your security key",
    ];

    async fn window(&self) -> Result<UiDriver> {
        let application = self.session.application();
        let (index, _) = application
            .wait_for_any_visible_name(&Self::HEADINGS, KEY_OPERATION_TIMEOUT)
            .await?;
        let dialog = application
            .wait_for_dialog(Self::HEADINGS[index], KEY_OPERATION_TIMEOUT)
            .await?;
        Ok(application.within(&dialog))
    }
}
impl SecurityKeyWindowDriver for LinuxSecurityKeyWindow<'_> {
    fn wait_open(&self) -> Result<()> {
        self.runtime.block_on(self.window())?;
        Ok(())
    }
    fn screenshot(&self) -> Result<PathBuf> {
        self.runtime.block_on(async {
            // GTK can replace the key selection/verification dialog while it is
            // being captured. Reacquire only the read-only snapshot.
            tokio::time::timeout(KEY_OPERATION_TIMEOUT, async {
                loop {
                    match self.window().await?.capture().await {
                        Ok(path) => return Ok(path),
                        Err(error) if is_removed_accessible(&error) => {
                            tracing::trace!(%error, "security key dialog was replaced before capture");
                        },
                        Err(error) => return Err(error),
                    }
                }
            }).await.context("timed out capturing the current Security Key prompt")?
        })
    }
    fn wait_closed(&self) -> Result<()> {
        self.runtime.block_on(async {
            let app = self.session.application();
            for name in Self::HEADINGS {
                app.wait_for_visible_absence(name, ELEMENT_TIMEOUT).await?;
            }
            Ok(())
        })
    }
}
