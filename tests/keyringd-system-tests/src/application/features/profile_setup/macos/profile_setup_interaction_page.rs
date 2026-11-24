use super::HEADING;
use crate::application::platform::macos::UiDriver;
use anyhow::Result;
pub struct ProfileSetupInteractionPage<'a> {
    pub(super) driver: UiDriver<'a>,
}
impl crate::application::features::profile_setup::ProfileSetupInteractionPageDriver
    for ProfileSetupInteractionPage<'_>
{
    type QrView<'a>
        = super::ProfileSetupQrView<'a>
    where
        Self: 'a;
    type PinView<'a>
        = super::ProfileSetupPinView<'a>
    where
        Self: 'a;
    type WaitingView<'a>
        = super::ProfileSetupWaitingView<'a>
    where
        Self: 'a;
    fn pin_view(&mut self) -> Result<Self::PinView<'_>> {
        Ok(super::ProfileSetupPinView {
            driver: self.driver.in_window(HEADING),
        })
    }
    fn waiting_view(&mut self) -> Result<Self::WaitingView<'_>> {
        Ok(super::ProfileSetupWaitingView {
            driver: self.driver.in_window(HEADING),
        })
    }
    fn content(
        &mut self,
    ) -> Result<crate::application::features::profile_setup::ProfileSetupProgress> {
        super::observe_progress(&self.driver)
    }
    fn qr_view(&mut self) -> Result<Self::QrView<'_>> {
        self.driver.wait_for("key-operation.qr")?;
        Ok(super::ProfileSetupQrView {
            driver: self.driver.in_window(HEADING),
        })
    }
}
