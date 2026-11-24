use crate::application::platform::linux::{ELEMENT_TIMEOUT, UiDriver};
use anyhow::Result;
use tokio::runtime::Runtime;

pub struct LinuxPasskeyNotFoundPage<'a> {
    pub(crate) runtime: &'a Runtime,
    pub(crate) session: UiDriver,
}

impl super::super::PasskeyNotFoundPageDriver for LinuxPasskeyNotFoundPage<'_> {
    fn continue_with_new_passkey(&mut self) -> Result<()> {
        self.runtime.block_on(async {
            self.session
                .select_radio_button("Create a new passkey instead", ELEMENT_TIMEOUT)
                .await?;
            let button = self
                .session
                .wait_for_visible_action("Continue", ELEMENT_TIMEOUT)
                .await?;
            self.session.click(&button).await?;
            self.session
                .wait_for_text_entry("Profile name", ELEMENT_TIMEOUT)
                .await?;
            Ok(())
        })
    }

    fn try_another_device(&mut self) -> Result<()> {
        self.runtime.block_on(async {
            self.session
                .wait_for_radio_checked("Try another device", ELEMENT_TIMEOUT)
                .await?;
            let button = self
                .session
                .wait_for_visible_action("Continue", ELEMENT_TIMEOUT)
                .await?;
            self.session.click(&button).await
        })
    }
}
