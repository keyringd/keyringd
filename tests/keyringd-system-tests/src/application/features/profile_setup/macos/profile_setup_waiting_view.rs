use super::HEADING;
use crate::application::platform::macos::UiDriver;
use anyhow::Result;

pub struct ProfileSetupWaitingView<'a> {
    pub(super) driver: UiDriver<'a>,
}
impl super::super::ProfileSetupWaitingViewDriver for ProfileSetupWaitingView<'_> {
    fn content(&self) -> Result<Vec<String>> {
        Ok(self
            .driver
            .find_element("profile-setup.prompt-title")?
            .filter(|element| element.hidden != Some(true))
            .and_then(|element| element.value)
            .into_iter()
            .collect())
    }
    fn screenshot(&self) -> Result<std::path::PathBuf> {
        self.driver.capture(HEADING)
    }
}
