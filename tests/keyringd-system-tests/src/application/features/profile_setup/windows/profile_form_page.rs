use super::super::ProfileFormPageDriver;
use crate::application::platform::windows::UiDriver;
use anyhow::{Context, Result};
use windows::Win32::UI::Accessibility::IUIAutomationElement;
pub struct ProfileFormPage<'a> {
    pub(super) driver: &'a UiDriver,
    pub(super) window: IUIAutomationElement,
}
impl ProfileFormPage<'_> {
    fn wait_create_enabled(&self, enabled: bool) -> Result<()> {
        self.driver
            .wait_visible(&self.window, "profile-setup-create")?;
        self.driver
            .wait_enabled(&self.window, "profile-setup-create", enabled)
    }
}
impl ProfileFormPageDriver for ProfileFormPage<'_> {
    fn enter_profile_name(&self, name: &str) -> Result<()> {
        self.driver
            .set_value(&self.window, "profile-setup-name", name)?;
        Ok(())
    }
    fn show_advanced_settings(&self) -> Result<()> {
        self.driver
            .expand(&self.window, "profile-setup-advanced-settings")?;
        self.driver
            .wait_visible(&self.window, "profile-setup-signature-format")
    }
    fn select_new_passkey(&self) -> Result<()> {
        self.driver
            .select_option(&self.window, "profile-setup-passkey-choice", 0)
    }
    fn select_existing_passkey(&self) -> Result<()> {
        self.driver
            .select_option(&self.window, "profile-setup-passkey-choice", 1)
    }
    fn submit(&mut self) -> Result<()> {
        // ValuePattern returns before the product callback updates button state.
        self.wait_create_enabled(true)?;
        self.driver.invoke(&self.window, "profile-setup-create")
    }
    fn content(&mut self) -> Result<super::super::ProfileSetupForm> {
        use windows::Win32::UI::Accessibility::{
            IUIAutomationSelectionItemPattern, UIA_SelectionItemPatternId,
        };
        self.driver
            .wait_visible(&self.window, "profile-setup-name")?;
        if self
            .driver
            .contains_automation_id(&self.window, "profile-setup-error")?
        {
            let error = self.driver.element(&self.window, "profile-setup-error")?;
            if !unsafe { error.CurrentName()? }.is_empty() {
                self.driver
                    .reveal_at_bottom(&self.window, "profile-setup-error")?;
            }
        }
        let choice = self.driver.element_named(
            &self.window,
            "Use an existing passkey\nUse an existing passkey to set up SSH on this computer",
        )?;
        let create = self.driver.element(&self.window, "profile-setup-create")?;
        unsafe {
            let selected = choice
                .GetCurrentPatternAs::<IUIAutomationSelectionItemPattern>(
                    UIA_SelectionItemPatternId,
                )?
                .CurrentIsSelected()?
                .as_bool();
            let new_choice = self.driver.element_named(
                &self.window,
                "Create a new passkey\nSet up Keyringd for the first time",
            )?;
            let new_selected = new_choice
                .GetCurrentPatternAs::<IUIAutomationSelectionItemPattern>(
                    UIA_SelectionItemPatternId,
                )?
                .CurrentIsSelected()?
                .as_bool();
            Ok(super::super::ProfileSetupForm {
                name: self.driver.text(&self.window, "profile-setup-name")?,
                existing_passkey: selected,
                new_passkey: new_selected,
                create_enabled: create.CurrentIsEnabled()?.as_bool(),
                create_label: create.CurrentName()?.to_string(),
                messages: self
                    .driver
                    .visible_texts(&self.window)
                    .context("observe profile setup visible text")?,
            })
        }
    }
}
