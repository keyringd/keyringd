use super::super::CompletionPageDriver;
use crate::application::platform::windows::UiDriver;
use anyhow::Result;
use windows::Win32::UI::Accessibility::IUIAutomationElement;
pub struct CompletionPage<'a> {
    pub(super) driver: &'a UiDriver,
    pub(super) window: IUIAutomationElement,
    pub(super) pid: u32,
}
impl CompletionPage<'_> {
    fn wait_closed(&self) -> Result<()> {
        self.driver
            .wait_window_absent(self.pid, "Add SSH profile")?;
        self.driver
            .wait_window_enabled(&self.driver.window_named(self.pid, "Keyringd")?, true)
    }
}
impl CompletionPageDriver for CompletionPage<'_> {
    fn content(&mut self) -> Result<super::super::ProfileSetupCompletion> {
        let done = self.driver.element(&self.window, "profile-setup-done")?;
        let trust = self
            .driver
            .element(&self.window, "profile-setup-public-keys")?;
        let description = self
            .driver
            .text(&self.window, "profile-setup-completed-description")?;
        unsafe {
            Ok(super::super::ProfileSetupCompletion {
                messages: vec![description],
                public_keys_enabled: trust.CurrentIsEnabled()?.as_bool(),
                done_enabled: done.CurrentIsEnabled()?.as_bool(),
            })
        }
    }
    fn done(&mut self) -> Result<()> {
        self.driver.invoke(&self.window, "profile-setup-done")?;
        self.wait_closed()
    }
}
