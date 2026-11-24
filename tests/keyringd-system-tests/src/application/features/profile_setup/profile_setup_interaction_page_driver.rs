use super::{ProfileSetupProgress, ProfileSetupQrViewDriver};
use anyhow::Result;

pub trait ProfileSetupInteractionPageDriver {
    type QrView<'a>: ProfileSetupQrViewDriver
    where
        Self: 'a;
    #[cfg(unix)]
    type PinView<'a>: super::ProfileSetupPinViewDriver
    where
        Self: 'a;
    #[cfg(unix)]
    type WaitingView<'a>: super::ProfileSetupWaitingViewDriver
    where
        Self: 'a;
    #[cfg(unix)]
    fn pin_view(&mut self) -> Result<Self::PinView<'_>>;
    #[cfg(unix)]
    fn waiting_view(&mut self) -> Result<Self::WaitingView<'_>>;
    fn content(&mut self) -> Result<ProfileSetupProgress>;
    fn qr_view(&mut self) -> Result<Self::QrView<'_>>;
}
