use crate::application::features::profiles::linux::LinuxProfilesWindow;
use crate::application::features::ssh_connection_test::{
    SshConnectionIdentity, SshConnectionTestWindowDriver,
};
use crate::application::platform::linux::{ELEMENT_TIMEOUT, UiDriver};
use anyhow::Result;
use std::path::PathBuf;
use tokio::runtime::Runtime;

pub struct LinuxSshConnectionTestWindow<'a> {
    pub(crate) runtime: &'a Runtime,
    pub(crate) session: UiDriver,
}
impl LinuxSshConnectionTestWindow<'_> {
    async fn window(&self) -> Result<UiDriver> {
        let session = self.session.application();
        let (dialog, _) = session
            .wait_for_dialog_action("Test", ELEMENT_TIMEOUT)
            .await?;
        Ok(session.within(&dialog))
    }
}
impl SshConnectionTestWindowDriver for LinuxSshConnectionTestWindow<'_> {
    fn open(&mut self, profile: &str) -> Result<()> {
        LinuxProfilesWindow::new(self.runtime, self.session.clone())?
            .open_profile_actions_menu(profile)?
            .activate("Test SSH connection…")?;
        self.runtime.block_on(async {
            self.session
                .application()
                .wait_for_dialog(&format!("Test SSH Connection — {profile}"), ELEMENT_TIMEOUT)
                .await?;
            Ok(())
        })
    }
    fn selected_identities(&self) -> Result<Vec<SshConnectionIdentity>> {
        self.runtime.block_on(async {
            let session = self.window().await?;
            let mut selected = Vec::new();
            for choice in [
                SshConnectionIdentity::DeviceCertificate,
                SshConnectionIdentity::DeviceKey,
                SshConnectionIdentity::Passkey,
            ] {
                let element = session.named(choice.choice_name()).action();
                if element.checked(ELEMENT_TIMEOUT).await? {
                    selected.push(choice);
                }
            }
            Ok(selected)
        })
    }
    fn select_identity(&self, identity: SshConnectionIdentity) -> Result<()> {
        self.runtime.block_on(async {
            let session = self.window().await?;
            // Invoke even an already selected choice: repeated activation must preserve exclusivity.
            let element = session
                .wait_for_visible_action(identity.choice_name(), ELEMENT_TIMEOUT)
                .await?;
            session.click(&element).await?;
            session
                .wait_for_radio_checked(identity.choice_name(), ELEMENT_TIMEOUT)
                .await?;
            Ok(())
        })
    }
    fn start(&self, host: &str) -> Result<()> {
        self.runtime.block_on(async {
            let session = self.window().await?;
            let host_entry = session
                .wait_for_text_entry("SSH destination", ELEMENT_TIMEOUT)
                .await?;
            session.replace_text(&host_entry, host).await?;
            let test = session
                .wait_for_enabled_action("Test", ELEMENT_TIMEOUT)
                .await?;
            session.click(&test).await
        })
    }
    fn screenshot(&self) -> Result<PathBuf> {
        self.runtime.block_on(async {
            let session = self.window().await?;
            session.capture().await
        })
    }
    fn close(&mut self) -> Result<()> {
        self.runtime.block_on(async {
            let session = self.window().await?;
            let close = session
                .wait_for_visible_action("Close", ELEMENT_TIMEOUT)
                .await?;
            session.click(&close).await?;
            self.session
                .application()
                .wait_for_visible_absence("SSH destination", ELEMENT_TIMEOUT)
                .await
        })
    }
}
