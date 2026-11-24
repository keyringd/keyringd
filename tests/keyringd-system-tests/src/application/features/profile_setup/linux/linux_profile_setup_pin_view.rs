use crate::application::platform::linux::{
    ELEMENT_TIMEOUT, Element, KEY_OPERATION_TIMEOUT, UiDriver,
};
use anyhow::{Result, ensure};
use std::time::{Duration, Instant};
use tokio::runtime::Runtime;

pub struct LinuxProfileSetupPinView<'a> {
    pub(crate) runtime: &'a Runtime,
    pub(crate) session: UiDriver,
}

impl super::super::ProfileSetupPinViewDriver for LinuxProfileSetupPinView<'_> {
    fn visible(&self, new_pin: bool) -> Result<bool> {
        self.runtime.block_on(async {
            if !self
                .session
                .visible_name_present(prompt_title(new_pin))
                .await?
            {
                return Ok(false);
            }
            Ok(self.session.visible_password_entry("PIN").await?.is_some())
        })
    }

    fn content(&self) -> Result<super::super::ProfileSetupPinContent> {
        self.runtime.block_on(async {
            let mut texts = self
                .session
                .visible_names_with_role(atspi::Role::Label)
                .await?;
            texts.extend(
                self.session
                    .visible_names_with_role(atspi::Role::Grouping)
                    .await?,
            );
            let pin = self.session.named("PIN").role(atspi::Role::PasswordText);
            let confirmation_enabled = if self.session.visible_name_present("Confirm PIN").await? {
                let field = self
                    .session
                    .named("Confirm PIN")
                    .role(atspi::Role::PasswordText);
                Some(field.enabled(ELEMENT_TIMEOUT).await?)
            } else {
                None
            };
            Ok(super::super::ProfileSetupPinContent {
                texts,
                pin_enabled: pin.enabled(ELEMENT_TIMEOUT).await?,
                confirmation_enabled,
                submit_visible: self
                    .session
                    .visible_action_enabled("Continue")
                    .await?
                    .is_some(),
            })
        })
    }

    fn screenshot(&self) -> Result<std::path::PathBuf> {
        self.runtime.block_on(self.session.capture())
    }

    fn enter(&mut self, value: &str, new_pin: bool) -> Result<()> {
        self.runtime.block_on(async {
            self.session
                .wait_for_group(prompt_title(new_pin), KEY_OPERATION_TIMEOUT)
                .await?;
            let pin = self
                .session
                .wait_for_password_entry("PIN", ELEMENT_TIMEOUT)
                .await?;
            self.session.replace_text(&pin, value).await?;
            if new_pin {
                let confirmation = self
                    .session
                    .wait_for_password_entry("Confirm PIN", ELEMENT_TIMEOUT)
                    .await?;
                self.session.replace_text(&confirmation, value).await?;
            }
            let submit = self
                .session
                .wait_for_enabled_action("Continue", ELEMENT_TIMEOUT)
                .await?;
            self.session.click(&submit).await?;
            self.wait_submitted(&pin).await
        })
    }
}

impl LinuxProfileSetupPinView<'_> {
    /// Observe the submitted prompt settling before another PIN request can be answered.
    async fn wait_submitted(&self, previous: &Element) -> Result<()> {
        let deadline = Instant::now() + ELEMENT_TIMEOUT;
        while self
            .session
            .visible_password_entry("PIN")
            .await?
            .is_some_and(|pin| pin.same_object(previous))
        {
            ensure!(
                Instant::now() < deadline,
                "submitted PIN prompt did not settle"
            );
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        Ok(())
    }
}

fn prompt_title(new_pin: bool) -> &'static str {
    if new_pin {
        "Set your security key PIN"
    } else {
        "Enter your security key PIN"
    }
}
