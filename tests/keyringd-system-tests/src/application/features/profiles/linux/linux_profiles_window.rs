use super::{
    LinuxProfileActionsMenu, LinuxProfileDetailsWindow, LinuxRemoveProfileWindow,
    LinuxTrustEntriesWindow,
};
use crate::application::features::profiles::ProfilesWindowDriver;
use crate::application::platform::linux::{ELEMENT_TIMEOUT, UiDriver};
use anyhow::Result;
use std::path::PathBuf;
use tokio::runtime::Runtime;

pub struct LinuxProfilesWindow<'a> {
    runtime: &'a Runtime,
    session: UiDriver,
}
impl<'a> LinuxProfilesWindow<'a> {
    pub(crate) fn new(runtime: &'a Runtime, session: UiDriver) -> Result<Self> {
        let session = runtime.block_on(async {
            let session = session
                .application()
                .wait_for_group("SSH Profiles", ELEMENT_TIMEOUT)
                .await?;
            session
                .wait_for_visible_action("Create Profile", ELEMENT_TIMEOUT)
                .await?;
            Ok::<_, anyhow::Error>(session)
        })?;
        Ok(Self { runtime, session })
    }
    pub fn close(&self) -> Result<()> {
        self.runtime.block_on(async {
            let app = self.session.application();
            let window = app.wait_for_window("Keyringd", ELEMENT_TIMEOUT).await?;
            let session = app.within(&window);
            let close = session
                .wait_for_visible_action("Close", ELEMENT_TIMEOUT)
                .await?;
            session.click(&close).await?;
            app.wait_for_visible_absence("SSH Profiles", ELEMENT_TIMEOUT)
                .await
        })
    }

    pub fn open_profile_actions_menu(&self, name: &str) -> Result<LinuxProfileActionsMenu<'_>> {
        let session = self.runtime.block_on(self.open_profile_actions(name))?;
        Ok(LinuxProfileActionsMenu {
            runtime: self.runtime,
            session,
            profile_name: name.to_owned(),
        })
    }
    async fn move_from_menu(&self, name: &str, action: &str) -> Result<()> {
        let menu = self.open_profile_actions(name).await?;
        let item = menu
            .wait_for_enabled_action(action, ELEMENT_TIMEOUT)
            .await?;
        menu.click(&item).await
    }
    async fn observe_profile_order(&self, first: &str, second: &str) -> Result<bool> {
        let first = self
            .session
            .named(&format!("Profile actions for {first}"))
            .action();
        let second = self
            .session
            .named(&format!("Profile actions for {second}"))
            .action();
        let first_y = first.extents(ELEMENT_TIMEOUT).await?.y;
        let second_y = second.extents(ELEMENT_TIMEOUT).await?.y;
        Ok(first_y < second_y)
    }
    async fn open_profile_actions(&self, name: &str) -> Result<UiDriver> {
        let actions = self
            .session
            .wait_for_enabled_action(&format!("Profile actions for {name}"), ELEMENT_TIMEOUT)
            .await?;
        self.session.click(&actions).await?;
        let session = self.session.application();
        let root = session
            .wait_for_visible_name(&format!("Profile actions menu for {name}"), ELEMENT_TIMEOUT)
            .await?;
        Ok(session.within(&root))
    }
}
impl ProfilesWindowDriver for LinuxProfilesWindow<'_> {
    fn creation_enabled(&self) -> Result<bool> {
        self.runtime
            .block_on(self.session.visible_action_enabled("Create Profile"))?
            .ok_or_else(|| anyhow::anyhow!("Create Profile is not visible"))
    }
    fn device_key_warning(&self) -> Result<String> {
        self.runtime.block_on(async {
            let deadline = std::time::Instant::now() + ELEMENT_TIMEOUT;
            loop {
                if let Some(message) = self
                    .session
                    .application()
                    .visible_names_with_role(atspi::Role::Label)
                    .await?
                    .into_iter()
                    .find(|text| text.starts_with("Hardware-backed Device Key storage"))
                {
                    return Ok(message);
                }
                anyhow::ensure!(
                    std::time::Instant::now() < deadline,
                    "hardware key warning did not appear"
                );
                tokio::time::sleep(std::time::Duration::from_millis(25)).await;
            }
        })
    }
    type ProfileDetailsWindow<'a>
        = LinuxProfileDetailsWindow<'a>
    where
        Self: 'a;
    type RemoveProfileWindow<'a>
        = LinuxRemoveProfileWindow<'a>
    where
        Self: 'a;
    type TrustEntriesWindow<'a>
        = LinuxTrustEntriesWindow<'a>
    where
        Self: 'a;
    fn move_profile_up(&self, name: &str) -> Result<()> {
        self.runtime.block_on(self.move_from_menu(name, "Move up"))
    }
    fn move_profile_down(&self, name: &str) -> Result<()> {
        self.runtime
            .block_on(self.move_from_menu(name, "Move down"))
    }
    fn profile_visible(&self, name: &str) -> Result<bool> {
        self.runtime.block_on(async {
            self.session
                .visible_name_present(&format!("Profile actions for {name}"))
                .await
        })
    }
    fn begin_rename(&self, name: &str) -> Result<()> {
        self.runtime.block_on(async {
            let menu = self.open_profile_actions(name).await?;
            let rename = menu
                .wait_for_visible_action("Rename", ELEMENT_TIMEOUT)
                .await?;
            self.session.click(&rename).await?;
            self.session
                .wait_for_visible_name(&format!("Profile name for {name}"), ELEMENT_TIMEOUT)
                .await?;
            Ok(())
        })
    }
    fn enter_profile_name(&self, name: &str, new_name: &str) -> Result<()> {
        self.runtime.block_on(async {
            let editor = self
                .session
                .wait_for_visible_name(&format!("Profile name for {name}"), ELEMENT_TIMEOUT)
                .await?;
            self.session.replace_text(&editor, new_name).await
        })
    }
    fn profile_name_editor_content(&self, name: &str) -> Result<String> {
        self.runtime.block_on(async {
            let editor = self.session.named(&format!("Profile name for {name}"));
            editor.text(ELEMENT_TIMEOUT).await
        })
    }
    fn confirm_rename(&self, name: &str) -> Result<()> {
        self.runtime.block_on(async {
            let editor = self
                .session
                .wait_for_visible_name(&format!("Profile name for {name}"), ELEMENT_TIMEOUT)
                .await?;
            self.session.click_then_press_key(&editor, "Return").await
        })
    }
    fn screenshot(&self) -> Result<PathBuf> {
        self.runtime.block_on(self.session.capture())
    }
    fn reorder_before(&self, source_name: &str, target_name: &str) -> Result<()> {
        self.runtime.block_on(async {
            let source = self
                .session
                .wait_for_visible_name(&format!("Drag {source_name} to reorder"), ELEMENT_TIMEOUT)
                .await?;
            let target = self
                .session
                .named(&format!("Profile actions for {target_name}"))
                .action();
            let target_rect = target.extents(ELEMENT_TIMEOUT).await?;
            let mut target_point = target_rect.center();
            target_point.y = target_rect.y;
            self.session.drag(&source, target_point).await
        })
    }
    fn profile_is_above(&self, first: &str, second: &str) -> Result<bool> {
        self.runtime
            .block_on(self.observe_profile_order(first, second))
    }
    fn toggle_profile(&self, name: &str) -> Result<()> {
        self.runtime.block_on(async {
            let toggle = self
                .session
                .wait_for_visible_name(&format!("Enable profile {name}"), ELEMENT_TIMEOUT)
                .await?;
            self.session.click(&toggle).await
        })
    }
    fn profile_enabled(&self, name: &str) -> Result<bool> {
        self.runtime.block_on(async {
            let toggle = self.session.named(&format!("Enable profile {name}"));
            toggle.checked(ELEMENT_TIMEOUT).await
        })
    }

    fn profile_present(&self, name: &str) -> Result<bool> {
        self.profile_visible(name)
    }
    fn profile_move_actions_enabled(&self, name: &str) -> Result<(bool, bool)> {
        self.runtime.block_on(async {
            let menu = self.open_profile_actions(name).await?;
            let result = async {
                let up = menu.named("Move up").action();
                let down = menu.named("Move down").action();
                Ok((
                    up.enabled(ELEMENT_TIMEOUT).await?,
                    down.enabled(ELEMENT_TIMEOUT).await?,
                ))
            }
            .await;
            LinuxProfileActionsMenu {
                runtime: self.runtime,
                session: menu,
                profile_name: name.to_owned(),
            }
            .dismiss()
            .await?;
            result
        })
    }
    fn open_details(&mut self, name: &str) -> Result<Self::ProfileDetailsWindow<'_>> {
        self.runtime.block_on(async {
            let menu = self.open_profile_actions(name).await?;
            let details = menu
                .wait_for_visible_action("Details…", ELEMENT_TIMEOUT)
                .await?;
            self.session.click(&details).await?;
            LinuxProfileDetailsWindow::wait_for(self.runtime, self.session.clone(), name).await
        })
    }
    fn open_trust_entries(&mut self, name: &str) -> Result<Self::TrustEntriesWindow<'_>> {
        self.runtime.block_on(async {
            let menu = self.open_profile_actions(name).await?;
            let trust_entries = menu
                .wait_for_visible_action("Public Keys…", ELEMENT_TIMEOUT)
                .await?;
            self.session.click(&trust_entries).await?;
            LinuxTrustEntriesWindow::wait_for(self.runtime, self.session.clone()).await
        })
    }
    fn open_removal(&mut self, name: &str) -> Result<Self::RemoveProfileWindow<'_>> {
        self.runtime.block_on(async {
            let menu = self.open_profile_actions(name).await?;
            let remove = menu
                .wait_for_visible_action("Remove profile…", ELEMENT_TIMEOUT)
                .await?;
            self.session.click(&remove).await?;
            LinuxRemoveProfileWindow::wait_for(self.runtime, self.session.clone(), name).await
        })
    }
}
