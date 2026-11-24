use crate::application::platform::linux::ELEMENT_TIMEOUT;
use crate::application::platform::linux::UiDriver;
use anyhow::Result;

pub struct MainMenu {
    session: UiDriver,
}

impl MainMenu {
    pub(in crate::application::features) async fn open(session: UiDriver) -> Result<Self> {
        let session = session.application();
        let menu = session
            .wait_for_toggle_button("Main Menu", ELEMENT_TIMEOUT)
            .await?;
        session.focus_with_tab(&menu, "^Keyringd$").await?;
        session.press_key_sequence("^Keyringd$", &["space"]).await?;
        let session = session
            .application()
            .wait_for_group("Application menu", ELEMENT_TIMEOUT)
            .await?;
        Ok(Self { session })
    }

    pub(crate) async fn open_about(self) -> Result<()> {
        let action = self
            .session
            .wait_for_enabled_action("About Keyringd", ELEMENT_TIMEOUT)
            .await?;
        self.session.click(&action).await?;
        Ok(())
    }
}
