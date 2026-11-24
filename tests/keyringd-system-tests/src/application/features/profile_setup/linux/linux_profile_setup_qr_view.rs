use crate::application::platform::linux::{ELEMENT_TIMEOUT, KEY_OPERATION_TIMEOUT, UiDriver};
use anyhow::Result;
use tokio::runtime::Runtime;

pub struct LinuxProfileSetupQrView<'a> {
    pub(crate) runtime: &'a Runtime,
    pub(crate) session: UiDriver,
}

impl super::super::ProfileSetupQrViewDriver for LinuxProfileSetupQrView<'_> {
    fn content(&self) -> Result<super::super::ProfileSetupProgress> {
        self.runtime
            .block_on(super::progress_observation::observe_progress(&self.session))
    }

    fn read_hybrid_qr(&self, previous: Option<&str>) -> Result<(String, std::path::PathBuf)> {
        self.runtime.block_on(async {
            self.session
                .wait_for_group("Authenticator transport choices", KEY_OPERATION_TIMEOUT)
                .await?
                .read_hybrid_qr(previous)
                .await
        })
    }

    fn press_use_security_key(&self) -> Result<()> {
        self.runtime.block_on(async {
            let button = self
                .session
                .wait_for_enabled_action("Use security key…", ELEMENT_TIMEOUT)
                .await?;
            self.session.click(&button).await
        })
    }
}
