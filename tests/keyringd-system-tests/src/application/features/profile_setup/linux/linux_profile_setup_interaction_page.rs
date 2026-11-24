use super::{LinuxProfileSetupPinView, LinuxProfileSetupQrView, LinuxProfileSetupWaitingView};
use crate::application::platform::linux::{ELEMENT_TIMEOUT, UiDriver};
use anyhow::Result;
use tokio::runtime::Runtime;

pub struct LinuxProfileSetupInteractionPage<'a> {
    pub(crate) runtime: &'a Runtime,
    pub(crate) session: UiDriver,
}

impl super::super::ProfileSetupInteractionPageDriver for LinuxProfileSetupInteractionPage<'_> {
    type QrView<'a>
        = LinuxProfileSetupQrView<'a>
    where
        Self: 'a;
    type PinView<'a>
        = LinuxProfileSetupPinView<'a>
    where
        Self: 'a;
    type WaitingView<'a>
        = LinuxProfileSetupWaitingView<'a>
    where
        Self: 'a;

    fn pin_view(&mut self) -> Result<Self::PinView<'_>> {
        Ok(LinuxProfileSetupPinView {
            runtime: self.runtime,
            session: self.session.clone(),
        })
    }

    fn waiting_view(&mut self) -> Result<Self::WaitingView<'_>> {
        Ok(LinuxProfileSetupWaitingView {
            runtime: self.runtime,
            session: self.session.clone(),
        })
    }

    fn content(&mut self) -> Result<super::super::ProfileSetupProgress> {
        self.runtime
            .block_on(super::progress_observation::observe_progress(&self.session))
    }

    fn qr_view(&mut self) -> Result<Self::QrView<'_>> {
        self.runtime.block_on(
            self.session
                .wait_for_group("Authenticator transport choices", ELEMENT_TIMEOUT),
        )?;
        Ok(LinuxProfileSetupQrView {
            runtime: self.runtime,
            session: self.session.clone(),
        })
    }
}
