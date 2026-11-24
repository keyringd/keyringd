use super::LinuxDeviceKeyProtectionWindow;
use crate::application::platform::linux::{ELEMENT_TIMEOUT, UiDriver};
use anyhow::Result;
use tokio::runtime::Runtime;

pub struct LinuxProfileActionsMenu<'a> {
    pub(super) runtime: &'a Runtime,
    pub(super) session: UiDriver,
    pub(super) profile_name: String,
}
impl LinuxProfileActionsMenu<'_> {
    pub fn action_enabled(&self, label: &str) -> Result<Option<bool>> {
        self.runtime
            .block_on(self.session.visible_action_enabled(label))
    }
    pub fn capture_screenshot(&self, name: &str) -> Result<()> {
        keyringd_testkit::record_image(name, self.runtime.block_on(self.session.capture())?)
    }
    pub fn close(&mut self) -> Result<()> {
        self.runtime.block_on(self.dismiss())
    }
    pub(super) async fn dismiss(&self) -> Result<()> {
        let session = self.session.application();
        let toggle = session
            .wait_for_enabled_action(
                &format!("Profile actions for {}", self.profile_name),
                ELEMENT_TIMEOUT,
            )
            .await?;
        session.click(&toggle).await?;
        session
            .wait_for_visible_absence(
                &format!("Profile actions menu for {}", self.profile_name),
                ELEMENT_TIMEOUT,
            )
            .await?;
        Ok(())
    }

    pub(crate) fn activate(&self, label: &str) -> Result<()> {
        self.runtime.block_on(async {
            let action = self
                .session
                .wait_for_visible_action(label, ELEMENT_TIMEOUT)
                .await?;
            self.session.click(&action).await
        })
    }
    pub fn open_password_protection(&mut self) -> Result<LinuxDeviceKeyProtectionWindow<'_>> {
        self.open_protection("Enable local authentication…", "Protect Device Key", "Save")
    }
    pub fn open_password_change(&mut self) -> Result<LinuxDeviceKeyProtectionWindow<'_>> {
        self.open_protection(
            "Change Device Key Password…",
            "Change Device Key Password",
            "Save",
        )
    }
    pub fn open_password_removal(&mut self) -> Result<LinuxDeviceKeyProtectionWindow<'_>> {
        self.open_protection(
            "Disable local authentication…",
            "Disable Password Protection",
            "Disable",
        )
    }

    fn open_protection(
        &self,
        action: &str,
        title: &str,
        submit: &str,
    ) -> Result<LinuxDeviceKeyProtectionWindow<'_>> {
        self.activate(action)?;
        self.runtime
            .block_on(LinuxDeviceKeyProtectionWindow::wait_for(
                self.runtime,
                self.session.clone(),
                title,
                submit,
            ))
    }
}
