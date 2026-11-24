use crate::application::features::profile_setup::ProfileFormPageDriver;
use crate::application::platform::macos::UiDriver;
use anyhow::Result;
use anyhow::ensure;
const NAME: &str = "profile-setup.name";
const CREATE: &str = "profile-setup.create";
const USE_EXISTING_PASSKEY: &str = "profile-setup.use-existing-passkey";
pub struct ProfileFormPage<'a> {
    pub(super) driver: UiDriver<'a>,
}
impl ProfileFormPageDriver for ProfileFormPage<'_> {
    fn enter_profile_name(&self, name: &str) -> Result<()> {
        let entered = self.driver.enter_text(NAME, name)?;
        ensure!(
            entered == name,
            "the profile name field holds {entered:?} after entering {name:?}"
        );
        Ok(())
    }
    fn submit(&mut self) -> Result<()> {
        self.driver.press(CREATE)
    }
    fn show_advanced_settings(&self) -> Result<()> {
        self.driver.press("profile-setup.advanced-settings")?;
        self.driver
            .wait_until("profile-setup.sk-format", "to become visible", |element| {
                element.hidden == Some(false)
            })?;
        Ok(())
    }
    fn content(&mut self) -> Result<super::super::ProfileSetupForm> {
        let create = self.driver.wait_for(CREATE)?;
        let existing = self.driver.wait_for(USE_EXISTING_PASSKEY)?.value;
        let new = self
            .driver
            .wait_for("profile-setup.create-new-passkey")?
            .value;
        let existing = match existing.as_deref() {
            Some("1") => true,
            Some("0") => false,
            other => anyhow::bail!("existing-passkey choice has invalid state: {other:?}"),
        };
        let new = match new.as_deref() {
            Some("1") => true,
            Some("0") => false,
            other => anyhow::bail!("new-passkey choice has invalid state: {other:?}"),
        };
        let mut messages = Vec::new();
        if let Some(status) = self.driver.find_element("profile-setup.status")?
            && status.hidden != Some(true)
            && let Some(value) = status.value
        {
            messages.push(value);
        }
        Ok(super::super::ProfileSetupForm {
            name: self
                .driver
                .wait_for(NAME)?
                .value
                .ok_or_else(|| anyhow::anyhow!("name has no value"))?,
            existing_passkey: existing,
            new_passkey: new,
            create_enabled: create
                .enabled
                .ok_or_else(|| anyhow::anyhow!("Create has no enabled state"))?,
            create_label: create
                .title
                .ok_or_else(|| anyhow::anyhow!("Create has no title"))?,
            messages,
        })
    }
    fn select_new_passkey(&self) -> Result<()> {
        self.driver.press("profile-setup.create-new-passkey")
    }

    fn select_existing_passkey(&self) -> Result<()> {
        self.driver.press(USE_EXISTING_PASSKEY)
    }
}
