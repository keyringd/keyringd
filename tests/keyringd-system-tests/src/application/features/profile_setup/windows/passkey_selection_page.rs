use crate::application::platform::windows::UiDriver;
use anyhow::Result;
use windows::Win32::UI::Accessibility::IUIAutomationElement;

pub struct PasskeySelectionPage<'a> {
    pub(super) driver: &'a UiDriver,
    pub(super) window: IUIAutomationElement,
}
impl super::super::PasskeySelectionPageDriver for PasskeySelectionPage<'_> {
    fn candidate_labels(&self) -> Result<Vec<String>> {
        let group = self
            .driver
            .element(&self.window, "profile-setup-passkey-candidates")?;
        self.driver.visible_radio_labels(&group)
    }
    fn select_passkey(&self, label: &str) -> Result<()> {
        self.driver.select_named(&self.window, label)
    }
    fn continue_recovery(&self) -> Result<()> {
        self.driver
            .invoke(&self.window, "profile-setup-select-passkey")
    }
}
