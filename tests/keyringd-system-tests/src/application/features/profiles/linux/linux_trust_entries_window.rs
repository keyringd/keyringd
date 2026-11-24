use crate::application::features::profiles::TrustEntriesWindowDriver;
use crate::application::platform::linux::{ELEMENT_TIMEOUT, UiDriver};
use anyhow::Result;
use std::path::PathBuf;
use tokio::runtime::Runtime;

pub struct LinuxTrustEntriesWindow<'a> {
    runtime: &'a Runtime,
    session: UiDriver,
}
impl<'a> LinuxTrustEntriesWindow<'a> {
    pub(super) async fn wait_for(runtime: &'a Runtime, session: UiDriver) -> Result<Self> {
        let session = session.application();
        let (dialog, _) = session
            .wait_for_dialog_action("Copy Certificate Authority Public Key", ELEMENT_TIMEOUT)
            .await?;
        Ok(Self {
            runtime,
            session: session.within(&dialog),
        })
    }
    fn copy_entry(&self, label: &str) -> Result<String> {
        self.runtime.block_on(async {
            let copy = self
                .session
                .wait_for_enabled_action(label, ELEMENT_TIMEOUT)
                .await?;
            self.session.copy_to_clipboard(&copy).await?;
            self.session.clipboard_text().await
        })
    }
}
impl TrustEntriesWindowDriver for LinuxTrustEntriesWindow<'_> {
    fn displayed_entries(&self) -> Result<Vec<String>> {
        self.runtime.block_on(async {
            let mut entries = self
                .session
                .visible_texts_with_role(atspi::Role::Text)
                .await?;
            entries.reverse();
            Ok(entries)
        })
    }
    fn notifications(&self) -> Result<Vec<String>> {
        self.runtime.block_on(async {
            self.session
                .application()
                .visible_names_with_role(atspi::Role::Label)
                .await
        })
    }
    fn screenshot(&self) -> Result<PathBuf> {
        self.runtime.block_on(self.session.capture())
    }
    fn close(&mut self) -> Result<()> {
        self.runtime.block_on(async {
            let close = self
                .session
                .wait_for_dialog_action("Close", ELEMENT_TIMEOUT)
                .await?
                .1;
            self.session.click(&close).await?;
            self.session
                .application()
                .wait_for_visible_absence("Copy Certificate Authority Public Key", ELEMENT_TIMEOUT)
                .await
        })
    }
    fn copy_ca_trust_entry(&self) -> Result<String> {
        self.copy_entry("Copy Certificate Authority Public Key")
    }
    fn copy_device_key_trust_entry(&self) -> Result<String> {
        self.copy_entry("Copy Device Key Public Key")
    }
    fn copy_passkey_trust_entry(&self) -> Result<String> {
        self.copy_entry("Copy Passkey Public Key")
    }
    fn copy_direct_key_trust_entries(&self) -> Result<String> {
        self.copy_entry("Copy Device + Passkey Public Keys")
    }
}
