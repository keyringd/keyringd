use crate::application::platform::linux::{ELEMENT_TIMEOUT, UiDriver};
use anyhow::Result;
use tokio::runtime::Runtime;

pub struct LinuxDeviceKeyProtectionWindow<'a> {
    runtime: &'a Runtime,
    session: UiDriver,
    title: String,
    submit: String,
}
impl<'a> LinuxDeviceKeyProtectionWindow<'a> {
    pub(super) async fn wait_for(
        runtime: &'a Runtime,
        session: UiDriver,
        title: &str,
        submit: &str,
    ) -> Result<Self> {
        let application = session.application();
        let dialog = application.wait_for_dialog(title, ELEMENT_TIMEOUT).await?;
        Ok(Self {
            runtime,
            session: application.within(&dialog),
            title: title.to_owned(),
            submit: submit.to_owned(),
        })
    }
    pub fn enter_password(&self, label: &str, value: &str) -> Result<()> {
        self.runtime.block_on(async {
            let field = self
                .session
                .wait_for_password_entry(label, ELEMENT_TIMEOUT)
                .await?;
            self.session.replace_text(&field, value).await
        })
    }
    pub fn submit(&self) -> Result<()> {
        self.runtime.block_on(async {
            let (_, button) = self
                .session
                .wait_for_dialog_action(&self.submit, ELEMENT_TIMEOUT)
                .await?;
            self.session.click(&button).await
        })
    }
    pub fn wait_closed(&self) -> Result<()> {
        self.runtime.block_on(async {
            self.session
                .application()
                .wait_for_absence(&self.title, ELEMENT_TIMEOUT)
                .await
        })
    }
    pub fn wait_for_notification(&self, text: &str) -> Result<()> {
        self.runtime.block_on(async {
            self.session
                .application()
                .wait_for_visible_name(text, ELEMENT_TIMEOUT)
                .await?;
            Ok(())
        })
    }

    pub fn capture_screenshot(&self, name: &str) -> Result<()> {
        keyringd_testkit::record_image(name, self.runtime.block_on(self.session.capture())?)
    }
}
