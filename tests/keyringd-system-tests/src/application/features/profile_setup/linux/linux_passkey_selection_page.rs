use crate::application::platform::linux::{ELEMENT_TIMEOUT, UiDriver};
use anyhow::Result;
use tokio::runtime::Runtime;

pub struct LinuxPasskeySelectionPage<'a> {
    pub(crate) runtime: &'a Runtime,
    pub(crate) session: UiDriver,
}

impl super::super::PasskeySelectionPageDriver for LinuxPasskeySelectionPage<'_> {
    fn candidate_labels(&self) -> Result<Vec<String>> {
        self.runtime.block_on(
            self.session
                .visible_names_with_role(atspi::Role::RadioButton),
        )
    }
    fn select_passkey(&self, label: &str) -> Result<()> {
        self.runtime
            .block_on(self.session.select_radio_button(label, ELEMENT_TIMEOUT))
    }
    fn continue_recovery(&self) -> Result<()> {
        self.runtime.block_on(async {
            let button = self
                .session
                .wait_for_enabled_action("Continue", ELEMENT_TIMEOUT)
                .await?;
            self.session.click(&button).await
        })
    }
}
