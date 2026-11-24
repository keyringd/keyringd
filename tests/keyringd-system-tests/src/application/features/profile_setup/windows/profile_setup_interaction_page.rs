use super::super::{ProfileSetupInteractionPageDriver, ProfileSetupProgress};
use crate::application::platform::windows::UiDriver;
use anyhow::Result;
use windows::Win32::UI::Accessibility::IUIAutomationElement;
pub struct ProfileSetupInteractionPage<'a> {
    pub(super) driver: &'a UiDriver,
    pub(super) window: IUIAutomationElement,
}
impl ProfileSetupInteractionPageDriver for ProfileSetupInteractionPage<'_> {
    type QrView<'a>
        = super::ProfileSetupQrView<'a>
    where
        Self: 'a;
    fn qr_view(&mut self) -> Result<Self::QrView<'_>> {
        Ok(super::ProfileSetupQrView {
            driver: self.driver,
            window: self.window.clone(),
        })
    }
    fn content(&mut self) -> Result<ProfileSetupProgress> {
        super::observe_profile_setup(self.driver, &self.window)
    }
}
