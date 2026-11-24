use super::super::PasskeyNotFoundPageDriver;
use crate::application::platform::windows::UiDriver;
use anyhow::Result;
use windows::Win32::UI::Accessibility::IUIAutomationElement;
pub struct PasskeyNotFoundPage<'a> {
    pub(super) driver: &'a UiDriver,
    pub(super) window: IUIAutomationElement,
}
impl PasskeyNotFoundPageDriver for PasskeyNotFoundPage<'_> {
    fn continue_with_new_passkey(&mut self) -> Result<()> {
        self.driver
            .select_option(&self.window, "profile-setup-not-found-choice", 1)?;
        self.driver
            .invoke(&self.window, "profile-setup-not-found-continue")
    }
    fn try_another_device(&mut self) -> Result<()> {
        self.driver
            .invoke(&self.window, "profile-setup-not-found-continue")
    }
}
