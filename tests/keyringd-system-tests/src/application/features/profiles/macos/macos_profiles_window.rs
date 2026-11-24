use std::path::PathBuf;

use anyhow::{Context, Result, ensure};

use crate::application::features::profile_setup::macos::MacosProfileSetupWizard;
use crate::application::features::profiles::macos::ProfileDetailsWindow;
use crate::application::features::profiles::macos::TrustEntriesWindow;
use crate::application::platform::macos::UiDriver;

const WINDOW_ANCHOR: &str = "profiles.configure-ssh-clients";
const LIST: &str = "profiles.list";
const ROW_PREFIX: &str = "profiles.row.";
const EMPTY_ADD: &str = "profiles.empty.add";
/// Native toolbar items expose itemIdentifier rather than an AX view identifier.
const TOOLBAR_ADD: &str = "com.keyringd.toolbar.add-profile";

/// The product Profiles window.
pub struct MacosProfilesWindow<'a> {
    driver: UiDriver<'a>,
}

impl<'a> MacosProfilesWindow<'a> {
    pub fn close(&self) -> Result<()> {
        self.driver.close_window(WINDOW_ANCHOR)?;
        self.driver.wait_absent(WINDOW_ANCHOR)
    }

    pub fn move_up(&self, name: &str) -> Result<()> {
        self.driver
            .choose_menu_item(&self.row(name, "actions")?, "profile-actions.move-up")
    }
    pub fn move_down(&self, name: &str) -> Result<()> {
        self.driver
            .choose_menu_item(&self.row(name, "actions")?, "profile-actions.move-down")
    }
    pub fn move_actions_enabled(&self, name: &str) -> Result<(bool, bool)> {
        let actions = self.row(name, "actions")?;
        let up = self.driver.menu_item(&actions, "profile-actions.move-up")?;
        let down = self
            .driver
            .menu_item(&actions, "profile-actions.move-down")?;
        anyhow::ensure!(
            up.hidden == Some(false) && down.hidden == Some(false),
            "move menu items must be visible"
        );
        Ok((
            up.enabled.context("Move up has no enabled state")?,
            down.enabled.context("Move down has no enabled state")?,
        ))
    }

    pub fn new(driver: &UiDriver<'a>) -> Result<Self> {
        let page = Self {
            driver: driver.in_window(WINDOW_ANCHOR),
        };
        page.driver.wait_for(WINDOW_ANCHOR)?;
        Ok(page)
    }

    pub fn capture_screenshot(&self, name: &str) -> anyhow::Result<()> {
        keyringd_testkit::record_image(name, self.screenshot()?)
    }
    pub fn screenshot(&self) -> Result<PathBuf> {
        self.driver.capture(WINDOW_ANCHOR)
    }

    /// Finds a row by its displayed name and returns its stable Profile ID.
    /// Duplicate names are ambiguous and must never select an arbitrary row.
    fn find_profile_id(&self, name: &str) -> Result<Option<String>> {
        let mut matches = Vec::new();
        for id in self.driver.identifiers(ROW_PREFIX)? {
            let Some(profile_id) = id
                .strip_prefix(ROW_PREFIX)
                .and_then(|id| id.strip_suffix(".name"))
            else {
                continue;
            };
            if let Some(element) = self.driver.find_element(&id)?
                && element.hidden != Some(true)
                && element.value.as_deref() == Some(name)
            {
                matches.push(profile_id.to_owned());
            }
        }
        ensure!(
            matches.len() <= 1,
            "multiple visible profiles named {name:?}"
        );
        Ok(matches.pop())
    }

    fn row(&self, name: &str, control: &str) -> Result<String> {
        self.driver
            .wait_for_observation(&format!("profile {name:?} to appear"), || {
                let id = self.find_profile_id(name)?;
                let observed = format!("matching Profile ID: {id:?}");
                Ok((id.map(|id| format!("{ROW_PREFIX}{id}.{control}")), observed))
            })
    }

    pub fn profile_visible(&self, name: &str) -> Result<bool> {
        Ok(self.find_profile_id(name)?.is_some())
    }

    /// Renames through the row menu and the inline field, the way the product
    /// asks a user to: choose Rename, type, then press Return.
    pub fn begin_rename(&self, from: &str) -> Result<()> {
        let field = self.row(from, "name")?;
        self.driver
            .choose_menu_item(&self.row(from, "actions")?, "profile-actions.rename")?;
        // The row is rebuilt with a text field under the same identifier.
        self.driver
            .wait_until(&field, "to become editable", |element| {
                element.role.as_deref() == Some("AXTextField")
            })?;
        Ok(())
    }

    fn rename_editor(&self) -> Result<String> {
        let mut editors = Vec::new();
        for id in self.driver.identifiers(ROW_PREFIX)? {
            if !id.ends_with(".name") {
                continue;
            }
            if self.driver.find_element(&id)?.is_some_and(|element| {
                element.hidden != Some(true) && element.role.as_deref() == Some("AXTextField")
            }) {
                editors.push(id);
            }
        }
        anyhow::ensure!(
            editors.len() == 1,
            "expected one inline profile name editor, found {editors:?}"
        );
        Ok(editors.remove(0))
    }
    pub fn enter_profile_name(&self, _name: &str, new_name: &str) -> Result<()> {
        self.driver.enter_text(&self.rename_editor()?, new_name)?;
        Ok(())
    }
    pub fn profile_name_editor_content(&self, _name: &str) -> Result<String> {
        self.driver
            .element(&self.rename_editor()?)?
            .value
            .context("rename editor has no value")
    }
    pub fn confirm_rename(&self, _name: &str) -> Result<()> {
        self.driver.press_return(&self.rename_editor()?)
    }

    pub fn open_details(&self, name: &str) -> Result<ProfileDetailsWindow<'a>> {
        self.driver
            .choose_menu_item(&self.row(name, "actions")?, "profile-actions.details")?;
        ProfileDetailsWindow::wait_for(&self.driver, name)
    }

    pub fn open_connection_test(&self, name: &str) -> Result<()> {
        self.driver.choose_menu_item(
            &self.row(name, "actions")?,
            "profile-actions.connection-test",
        )?;
        self.driver
            .in_window("connection-test.host")
            .wait_for("connection-test.host")?;
        Ok(())
    }

    pub fn open_trust_entries(&self, name: &str) -> Result<TrustEntriesWindow<'a>> {
        self.driver
            .choose_menu_item(&self.row(name, "actions")?, "profile-actions.trust-entries")?;
        TrustEntriesWindow::wait_for(&self.driver, name)
    }

    /// The switch reports its state as "1" or "0" through accessibility.
    pub fn profile_enabled(&self, name: &str) -> Result<bool> {
        let field = self.row(name, "enabled")?;
        match self.driver.element(&field)?.value.as_deref() {
            Some("1") => Ok(true),
            Some("0") => Ok(false),
            other => anyhow::bail!("unexpected profile switch value: {other:?}"),
        }
    }

    pub fn toggle_profile(&self, name: &str) -> Result<()> {
        self.driver.press(&self.row(name, "enabled")?)
    }

    /// The profile names in the order the list shows them.
    pub fn profile_order(&self) -> Result<Vec<String>> {
        let mut names = Vec::new();
        for id in self.driver.identifiers(ROW_PREFIX)? {
            if id.ends_with(".name")
                && let Some(element) = self.driver.find_element(&id)?
                && element.hidden != Some(true)
            {
                names.push(
                    element
                        .value
                        .context("profile name has no accessibility value")?,
                );
            }
        }
        Ok(names)
    }

    pub fn profile_is_above(&self, upper: &str, lower: &str) -> Result<bool> {
        let order = self.profile_order()?;
        let position = |name: &str| {
            order
                .iter()
                .position(|profile| profile == name)
                .with_context(|| format!("the list does not show {name:?}; it shows {order:?}"))
        };
        Ok(position(upper)? < position(lower)?)
    }

    /// Moves `moved` above `target` by dropping it there, the way dragging the
    /// row does. The list order is read back, because a drop the product
    /// refuses would otherwise pass unnoticed.
    pub fn reorder_before(&self, moved: &str, target: &str) -> Result<()> {
        let order = self.profile_order()?;
        let position = |name: &str| {
            order
                .iter()
                .position(|profile| profile == name)
                .with_context(|| format!("the list does not show {name:?}; it shows {order:?}"))
        };
        self.driver
            .drag_row(LIST, position(moved)?, position(target)?)?;
        Ok(())
    }

    pub fn open_device_key_protection(
        &self,
        name: &str,
        required: bool,
    ) -> Result<super::DeviceKeyProtectionWindow<'a>> {
        let item = if required {
            "profile-actions.require-device-key-authorization"
        } else {
            "profile-actions.disable-device-key-authorization"
        };
        self.driver
            .choose_menu_item(&self.row(name, "actions")?, item)?;
        super::DeviceKeyProtectionWindow::wait_for(&self.driver)
    }

    /// Opens the creation wizard the way the window offers it: the empty-state
    /// action while there is no profile, the toolbar item afterwards. The
    /// sheet appearing is the confirmation, because a press reports success
    /// unreliably.
    pub fn start_profile_creation(&self) -> Result<MacosProfileSetupWizard<'a>> {
        if self
            .driver
            .find_element(EMPTY_ADD)?
            .is_some_and(|element| element.hidden != Some(true))
        {
            self.driver
                .wait_until(EMPTY_ADD, "to be enabled", |element| {
                    element.enabled == Some(true)
                })?;
            self.driver.press(EMPTY_ADD)?;
        } else {
            self.driver
                .invoke_toolbar_item(WINDOW_ANCHOR, TOOLBAR_ADD)?;
        }
        MacosProfileSetupWizard::wait_for(&self.driver)
    }
}

impl crate::application::features::profiles::ProfilesWindowDriver for MacosProfilesWindow<'_> {
    fn creation_enabled(&self) -> Result<bool> {
        self.driver
            .wait_for(EMPTY_ADD)?
            .enabled
            .context("Create Profile has no enabled state")
    }
    fn device_key_warning(&self) -> Result<String> {
        self.driver
            .wait_for("profiles.hardware-warning")?
            .value
            .context("hardware key warning has no text")
    }
    type ProfileDetailsWindow<'a>
        = super::ProfileDetailsWindow<'a>
    where
        Self: 'a;
    type RemoveProfileWindow<'a>
        = super::RemoveProfileWindow<'a>
    where
        Self: 'a;
    type TrustEntriesWindow<'a>
        = super::TrustEntriesWindow<'a>
    where
        Self: 'a;

    fn begin_rename(&self, name: &str) -> Result<()> {
        MacosProfilesWindow::begin_rename(self, name)
    }
    fn enter_profile_name(&self, name: &str, new_name: &str) -> Result<()> {
        MacosProfilesWindow::enter_profile_name(self, name, new_name)
    }
    fn profile_name_editor_content(&self, name: &str) -> Result<String> {
        MacosProfilesWindow::profile_name_editor_content(self, name)
    }
    fn confirm_rename(&self, name: &str) -> Result<()> {
        MacosProfilesWindow::confirm_rename(self, name)
    }
    fn screenshot(&self) -> Result<PathBuf> {
        MacosProfilesWindow::screenshot(self)
    }
    fn move_profile_up(&self, name: &str) -> Result<()> {
        self.move_up(name)
    }
    fn move_profile_down(&self, name: &str) -> Result<()> {
        self.move_down(name)
    }
    fn profile_move_actions_enabled(&self, name: &str) -> Result<(bool, bool)> {
        self.move_actions_enabled(name)
    }
    fn reorder_before(&self, source: &str, target: &str) -> Result<()> {
        MacosProfilesWindow::reorder_before(self, source, target)
    }
    fn profile_is_above(&self, upper: &str, lower: &str) -> Result<bool> {
        MacosProfilesWindow::profile_is_above(self, upper, lower)
    }
    fn profile_present(&self, name: &str) -> Result<bool> {
        Ok(self.find_profile_id(name)?.is_some())
    }
    fn profile_visible(&self, name: &str) -> Result<bool> {
        MacosProfilesWindow::profile_visible(self, name)
    }
    fn toggle_profile(&self, name: &str) -> Result<()> {
        MacosProfilesWindow::toggle_profile(self, name)
    }
    fn profile_enabled(&self, name: &str) -> Result<bool> {
        MacosProfilesWindow::profile_enabled(self, name)
    }
    fn open_details(&mut self, name: &str) -> Result<Self::ProfileDetailsWindow<'_>> {
        MacosProfilesWindow::open_details(self, name)
    }
    fn open_removal(&mut self, name: &str) -> Result<Self::RemoveProfileWindow<'_>> {
        self.driver
            .choose_menu_item(&self.row(name, "actions")?, "profile-actions.remove")?;
        super::RemoveProfileWindow::wait_for(&self.driver)
    }
    fn open_trust_entries(&mut self, name: &str) -> Result<Self::TrustEntriesWindow<'_>> {
        MacosProfilesWindow::open_trust_entries(self, name)
    }
}
