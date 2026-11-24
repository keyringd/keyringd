use crate::application::features::profile_setup::PasskeyNotFoundPageDriver;
use anyhow::Result;

use crate::application::platform::macos::UiDriver;

pub struct PasskeyNotFoundPage<'a> {
    pub(super) driver: UiDriver<'a>,
}

impl PasskeyNotFoundPageDriver for PasskeyNotFoundPage<'_> {
    fn continue_with_new_passkey(&mut self) -> Result<()> {
        self.driver
            .press("profile-setup.create-new-passkey-instead")?;
        self.driver
            .wait_value("profile-setup.create-new-passkey-instead", "1")?;
        self.driver.press("profile-setup.not-found-continue")
    }
    fn try_another_device(&mut self) -> Result<()> {
        self.driver.wait_for("profile-setup.try-another-device")?;
        self.driver.press("profile-setup.try-another-device")?;
        self.driver
            .wait_value("profile-setup.try-another-device", "1")?;
        self.driver.press("profile-setup.not-found-continue")
    }
}
