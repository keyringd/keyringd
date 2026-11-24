use crate::application::platform::linux::{ELEMENT_TIMEOUT, UiDriver};
use anyhow::Result;
use tokio::runtime::Runtime;

const SUBMIT_LABELS: [&str; 2] = ["Create", "Try again"];

pub struct LinuxProfileFormPage<'a> {
    pub(crate) runtime: &'a Runtime,
    pub(crate) session: UiDriver,
}

impl super::super::ProfileFormPageDriver for LinuxProfileFormPage<'_> {
    fn enter_profile_name(&self, name: &str) -> Result<()> {
        self.runtime.block_on(async {
            let field = self
                .session
                .wait_for_text_entry("Profile name", ELEMENT_TIMEOUT)
                .await?;
            self.session.replace_text(&field, name).await
        })
    }

    fn content(&mut self) -> Result<super::super::ProfileSetupForm> {
        self.runtime.block_on(async {
            let field = self.session.named("Profile name").role(atspi::Role::Text);
            let existing = self
                .session
                .named("Use an existing passkey")
                .role(atspi::Role::RadioButton);
            let new = self
                .session
                .named("Create a new passkey")
                .role(atspi::Role::RadioButton);
            let create = self
                .session
                .wait_for_any_visible_action(&SUBMIT_LABELS, ELEMENT_TIMEOUT)
                .await?;
            let (create_label, create_enabled) = self.session.action_status(&create).await?;
            Ok(super::super::ProfileSetupForm {
                name: field.text(ELEMENT_TIMEOUT).await?,
                existing_passkey: existing.checked(ELEMENT_TIMEOUT).await?,
                new_passkey: new.checked(ELEMENT_TIMEOUT).await?,
                create_enabled,
                create_label,
                messages: self
                    .session
                    .visible_names_with_role(atspi::Role::Label)
                    .await?,
            })
        })
    }

    fn show_advanced_settings(&self) -> Result<()> {
        self.runtime.block_on(async {
            let expander = self
                .session
                .wait_for_visible_action("Advanced settings", ELEMENT_TIMEOUT)
                .await?;
            self.session.click(&expander).await?;
            self.session
                .wait_for_visible_name("SSH authentication signature format", ELEMENT_TIMEOUT)
                .await?;
            Ok(())
        })
    }

    fn select_new_passkey(&self) -> Result<()> {
        self.runtime.block_on(
            self.session
                .select_radio_button("Create a new passkey", ELEMENT_TIMEOUT),
        )
    }

    fn select_existing_passkey(&self) -> Result<()> {
        self.runtime.block_on(
            self.session
                .select_radio_button("Use an existing passkey", ELEMENT_TIMEOUT),
        )
    }

    fn submit(&mut self) -> Result<()> {
        self.runtime.block_on(async {
            let create = self
                .session
                .wait_for_any_visible_action(&SUBMIT_LABELS, ELEMENT_TIMEOUT)
                .await?;
            self.session.click(&create).await
        })
    }
}
