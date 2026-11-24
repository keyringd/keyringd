use super::LinuxProfileSetupWizard;
use crate::application::platform::linux::{ELEMENT_TIMEOUT, UiDriver};
use anyhow::Result;
use tokio::runtime::Runtime;

pub struct LinuxProfileSetupWindow<'a> {
    pub(crate) runtime: &'a Runtime,
    pub(crate) session: UiDriver,
}

impl super::super::ProfileSetupWindowDriver for LinuxProfileSetupWindow<'_> {
    type Wizard<'a>
        = LinuxProfileSetupWizard<'a>
    where
        Self: 'a;

    fn screenshot(&self) -> Result<std::path::PathBuf> {
        self.runtime.block_on(self.session.capture())
    }

    fn open(&mut self) -> Result<()> {
        self.runtime.block_on(async {
            let profiles = self
                .session
                .application()
                .wait_for_group("SSH Profiles", ELEMENT_TIMEOUT)
                .await?;
            let create = profiles
                .wait_for_enabled_action("Create Profile", ELEMENT_TIMEOUT)
                .await?;
            profiles.click(&create).await?;
            self.session
                .application()
                .wait_for_group("Profile setup", ELEMENT_TIMEOUT)
                .await?;
            Ok(())
        })
    }

    fn close(&mut self) -> Result<()> {
        self.runtime.block_on(async {
            let application = self.session.application();
            let window = application
                .wait_for_dialog("Create SSH Profile", ELEMENT_TIMEOUT)
                .await?;
            let session = application.within(&window);
            let cancel = session
                .wait_for_visible_action("Cancel", ELEMENT_TIMEOUT)
                .await?;
            session.click(&cancel).await?;
            session
                .application()
                .wait_for_visible_absence("Create SSH Profile", ELEMENT_TIMEOUT)
                .await
        })
    }

    fn wizard(&mut self) -> Result<Self::Wizard<'_>> {
        let session = self.runtime.block_on(async {
            let application = self.session.application();
            let window = application
                .wait_for_dialog("Create SSH Profile", ELEMENT_TIMEOUT)
                .await?;
            Ok::<_, anyhow::Error>(application.within(&window))
        })?;
        Ok(LinuxProfileSetupWizard {
            runtime: self.runtime,
            session,
        })
    }
}
