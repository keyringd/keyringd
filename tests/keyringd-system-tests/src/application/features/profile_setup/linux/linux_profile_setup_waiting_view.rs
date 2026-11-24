use crate::application::platform::linux::UiDriver;
use anyhow::Result;
use tokio::runtime::Runtime;

pub struct LinuxProfileSetupWaitingView<'a> {
    pub(crate) runtime: &'a Runtime,
    pub(crate) session: UiDriver,
}

impl super::super::ProfileSetupWaitingViewDriver for LinuxProfileSetupWaitingView<'_> {
    fn content(&self) -> Result<Vec<String>> {
        Ok(self
            .runtime
            .block_on(super::progress_observation::observe_progress(&self.session))?
            .texts)
    }

    fn screenshot(&self) -> Result<std::path::PathBuf> {
        self.runtime.block_on(self.session.capture())
    }
}
