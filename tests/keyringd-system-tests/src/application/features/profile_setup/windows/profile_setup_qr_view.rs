use super::super::ProfileSetupQrViewDriver;
use crate::application::platform::windows::UiDriver;
use anyhow::Result;
use windows::Win32::UI::Accessibility::IUIAutomationElement;
pub struct ProfileSetupQrView<'a> {
    pub(super) driver: &'a UiDriver,
    pub(super) window: IUIAutomationElement,
}
impl ProfileSetupQrViewDriver for ProfileSetupQrView<'_> {
    fn content(&self) -> Result<super::super::ProfileSetupProgress> {
        super::observe_profile_setup(self.driver, &self.window)
    }
    fn read_hybrid_qr(&self, previous: Option<&str>) -> Result<(String, std::path::PathBuf)> {
        self.driver.read_hybrid_qr(&self.window, previous)
    }

    fn press_use_security_key(&self) -> Result<()> {
        self.driver.invoke_named(&self.window, "Use security key…")
    }
}
