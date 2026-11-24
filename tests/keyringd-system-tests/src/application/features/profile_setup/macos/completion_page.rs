use super::HEADING;
use crate::application::features::profile_setup::CompletionPageDriver;
use crate::application::platform::macos::UiDriver;
use anyhow::Result;
const DONE: &str = "profile-setup.done";
pub struct CompletionPage<'a> {
    pub(super) driver: UiDriver<'a>,
}
impl CompletionPageDriver for CompletionPage<'_> {
    fn content(&mut self) -> Result<super::super::ProfileSetupCompletion> {
        let purpose = self.driver.wait_for("profile-setup.purpose")?;
        let trust = self.driver.wait_for("profile-setup.trust-entries")?;
        let done = self.driver.wait_for(DONE)?;
        Ok(super::super::ProfileSetupCompletion {
            messages: vec![
                purpose
                    .value
                    .ok_or_else(|| anyhow::anyhow!("purpose has no value"))?,
            ],
            public_keys_enabled: trust
                .enabled
                .ok_or_else(|| anyhow::anyhow!("Public Keys has no enabled state"))?,
            done_enabled: done
                .enabled
                .ok_or_else(|| anyhow::anyhow!("Done has no enabled state"))?,
        })
    }
    fn done(&mut self) -> Result<()> {
        self.driver.wait_until(DONE, "to be available", |element| {
            element.enabled == Some(true)
        })?;
        self.driver.press(DONE)?;
        self.driver.wait_absent(HEADING)
    }
}
