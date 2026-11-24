use crate::application::platform::linux::{ELEMENT_TIMEOUT, UiDriver};
use anyhow::Result;
use tokio::runtime::Runtime;

pub struct LinuxCompletionPage<'a> {
    pub(crate) runtime: &'a Runtime,
    pub(crate) session: UiDriver,
}

impl super::super::CompletionPageDriver for LinuxCompletionPage<'_> {
    fn content(&mut self) -> Result<super::super::ProfileSetupCompletion> {
        self.runtime.block_on(async {
            let trust = self.session.named("Public Keys").action();
            let done = self.session.named("Done").action();
            let messages = self
                .session
                .visible_names_with_role(atspi::Role::Label)
                .await?;
            Ok(super::super::ProfileSetupCompletion {
                messages,
                public_keys_enabled: trust.enabled(ELEMENT_TIMEOUT).await?,
                done_enabled: done.enabled(ELEMENT_TIMEOUT).await?,
            })
        })
    }

    fn done(&mut self) -> Result<()> {
        self.runtime.block_on(async {
            let done = self
                .session
                .wait_for_enabled_action("Done", ELEMENT_TIMEOUT)
                .await?;
            self.session.click(&done).await?;
            self.session
                .application()
                .wait_for_visible_absence("Create SSH Profile", ELEMENT_TIMEOUT)
                .await
        })
    }
}
