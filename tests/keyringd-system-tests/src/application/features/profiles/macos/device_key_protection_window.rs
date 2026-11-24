use crate::application::platform::macos::UiDriver;
use anyhow::Result;
pub struct DeviceKeyProtectionWindow<'a> {
    driver: UiDriver<'a>,
}
impl<'a> DeviceKeyProtectionWindow<'a> {
    pub(super) fn wait_for(driver: &UiDriver<'a>) -> Result<Self> {
        let driver = driver.in_sheet("profiles.configure-ssh-clients");
        driver.wait_for("device-key-protection.confirm")?;
        Ok(Self { driver })
    }
    pub fn capture_screenshot(&self, name: &str) -> anyhow::Result<()> {
        keyringd_testkit::record_image(name, self.screenshot()?)
    }
    pub fn screenshot(&self) -> Result<std::path::PathBuf> {
        self.driver.capture_current_window()
    }
    pub fn qr_prompt(
        &self,
    ) -> Result<crate::application::features::qr_prompt_window::macos::MacosQrPromptWindow<'a>>
    {
        use crate::application::features::qr_prompt_window::QrPromptWindowDriver;
        let window =
            crate::application::features::qr_prompt_window::macos::MacosQrPromptWindow::new(
                &self.driver,
            );
        window.security_key_enabled()?;
        Ok(window)
    }
    pub fn trust_entries_window(&self, name: &str) -> Result<super::TrustEntriesWindow<'a>> {
        super::TrustEntriesWindow::wait_for(&self.driver, name)
    }
    pub fn confirm(&self) -> Result<()> {
        self.driver.press("device-key-protection.confirm")
    }
}
