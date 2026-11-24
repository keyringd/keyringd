use super::{LinuxReplaceDeviceKeyWindow, LinuxTrustEntriesWindow};
use crate::application::features::profiles::{ProfileDetails, ProfileDetailsWindowDriver};
use crate::application::platform::linux::{ELEMENT_TIMEOUT, UiDriver};
use anyhow::Result;
use std::path::PathBuf;
use tokio::runtime::Runtime;

pub struct LinuxProfileDetailsWindow<'a> {
    runtime: &'a Runtime,
    session: UiDriver,
    title: String,
}

impl<'a> LinuxProfileDetailsWindow<'a> {
    pub(super) async fn wait_for(
        runtime: &'a Runtime,
        session: UiDriver,
        profile_name: &str,
    ) -> Result<Self> {
        let session = session.application();
        let title = format!("Details for “{profile_name}”");
        let dialog = session.wait_for_dialog(&title, ELEMENT_TIMEOUT).await?;
        Ok(Self {
            runtime,
            session: session.within(&dialog),
            title,
        })
    }
    async fn detail_value(&self, field: &str) -> Result<String> {
        let mut values = self
            .session
            .containing_row(field)
            .await?
            .visible_names_with_role(atspi::Role::Label)
            .await?;
        values.retain(|value| !value.is_empty() && value != field);
        anyhow::ensure!(
            values.len() == 1,
            "{field} must have one displayed value: {values:?}"
        );
        Ok(values.remove(0))
    }
}

impl ProfileDetailsWindowDriver for LinuxProfileDetailsWindow<'_> {
    type TrustEntriesWindow<'a>
        = LinuxTrustEntriesWindow<'a>
    where
        Self: 'a;
    type ReplaceDeviceKeyWindow<'a>
        = LinuxReplaceDeviceKeyWindow<'a>
    where
        Self: 'a;
    fn content(&self) -> Result<ProfileDetails> {
        self.runtime.block_on(async {
            Ok(ProfileDetails {
                status: self.detail_value("Status").await?,
                signature_algorithm: self
                    .detail_value("SSH authentication signature format")
                    .await?,
                protection: self.detail_value("Local authentication").await?,
            })
        })
    }
    fn screenshot(&self) -> Result<PathBuf> {
        self.runtime.block_on(self.session.capture())
    }
    fn close(&mut self) -> Result<()> {
        self.runtime.block_on(async {
            let dialog = self
                .session
                .wait_for_dialog_action("Close", ELEMENT_TIMEOUT)
                .await?
                .0;
            let close = self
                .session
                .wait_for_action_within(&dialog, "Close", ELEMENT_TIMEOUT)
                .await?;
            self.session.click(&close).await?;
            self.session
                .application()
                .wait_for_absence(&self.title, ELEMENT_TIMEOUT)
                .await?;
            Ok(())
        })
    }
    fn open_trust_entries(&mut self) -> Result<Self::TrustEntriesWindow<'_>> {
        self.runtime.block_on(async {
            let open = self
                .session
                .wait_for_dialog_action("Public Keys…", ELEMENT_TIMEOUT)
                .await?
                .1;
            self.session.click(&open).await?;
            LinuxTrustEntriesWindow::wait_for(self.runtime, self.session.clone()).await
        })
    }
    fn open_replacement(&mut self) -> Result<Self::ReplaceDeviceKeyWindow<'_>> {
        self.runtime.block_on(async {
            let replace = self
                .session
                .wait_for_dialog_action("Replace…", ELEMENT_TIMEOUT)
                .await?
                .1;
            self.session.click(&replace).await?;
            LinuxReplaceDeviceKeyWindow::wait_for(self.runtime, self.session.clone()).await
        })
    }
}
