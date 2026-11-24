use super::LinuxTrustEntriesWindow;
use crate::application::features::profiles::{
    DeviceKeyReplacementResult, ReplaceDeviceKeyWindowDriver,
};
use crate::application::features::qr_prompt_window::QrPromptWindowDriver;
use crate::application::features::qr_prompt_window::linux::LinuxQrPromptWindow;
use crate::application::platform::linux::{ELEMENT_TIMEOUT, KEY_OPERATION_TIMEOUT, UiDriver};
use anyhow::Result;
use std::path::PathBuf;
use tokio::runtime::Runtime;

pub struct LinuxReplaceDeviceKeyWindow<'a> {
    runtime: &'a Runtime,
    session: UiDriver,
}
impl<'a> LinuxReplaceDeviceKeyWindow<'a> {
    pub(super) async fn wait_for(runtime: &'a Runtime, session: UiDriver) -> Result<Self> {
        let session = session.application();
        let dialog = session
            .wait_for_dialog("Replace Device Key?", ELEMENT_TIMEOUT)
            .await?;
        Ok(Self {
            runtime,
            session: session.within(&dialog),
        })
    }
}
impl ReplaceDeviceKeyWindowDriver for LinuxReplaceDeviceKeyWindow<'_> {
    type TrustEntriesWindow<'a>
        = LinuxTrustEntriesWindow<'a>
    where
        Self: 'a;
    fn confirm(&self) -> Result<()> {
        self.runtime.block_on(async {
            let replace = self
                .session
                .wait_for_dialog_action("Replace", ELEMENT_TIMEOUT)
                .await?
                .1;
            self.session.click(&replace).await
        })
    }
    fn press_use_security_key(&self) -> Result<()> {
        LinuxQrPromptWindow {
            runtime: self.runtime,
            session: self.session.clone(),
        }
        .press_use_security_key()
    }
    fn result_visible(&self) -> Result<bool> {
        self.runtime.block_on(async {
            self.session
                .application()
                .visible_name_present("Device Key Replaced")
                .await
        })
    }
    fn result(&self) -> Result<DeviceKeyReplacementResult> {
        self.runtime.block_on(async {
            let dialog = self
                .session
                .application()
                .wait_for_dialog("Device Key Replaced", KEY_OPERATION_TIMEOUT)
                .await?;
            let session = self.session.within(&dialog);
            let public_keys = session.named("Public Keys…").action();
            let close = session.named("Close").action();
            Ok(DeviceKeyReplacementResult {
                messages: session.visible_names_with_role(atspi::Role::Label).await?,
                public_keys_enabled: public_keys.enabled(ELEMENT_TIMEOUT).await?,
                close_enabled: close.enabled(ELEMENT_TIMEOUT).await?,
            })
        })
    }
    fn screenshot(&self) -> Result<PathBuf> {
        self.runtime.block_on(self.session.capture())
    }
    fn open_trust_entries(&mut self, _name: &str) -> Result<Self::TrustEntriesWindow<'_>> {
        self.runtime.block_on(async {
            let dialog = self
                .session
                .application()
                .wait_for_dialog("Device Key Replaced", ELEMENT_TIMEOUT)
                .await?;
            let open = self
                .session
                .wait_for_action_within(&dialog, "Public Keys…", ELEMENT_TIMEOUT)
                .await?;
            self.session.click(&open).await?;
            let trust_entries =
                LinuxTrustEntriesWindow::wait_for(self.runtime, self.session.clone()).await?;
            self.session
                .application()
                .wait_for_visible_absence("Device Key Replaced", ELEMENT_TIMEOUT)
                .await?;
            Ok(trust_entries)
        })
    }
}
