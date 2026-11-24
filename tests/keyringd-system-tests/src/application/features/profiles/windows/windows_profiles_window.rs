use crate::application::features::profile_setup::windows::WindowsProfileSetupWizard;
use crate::application::features::profiles::windows::ProfileDetailsWindow;
use crate::application::features::profiles::windows::RemoveProfileWindow;
use crate::application::features::profiles::windows::TrustEntriesWindow;
use crate::application::platform::windows::UiDriver;
use anyhow::Result;
use windows::Win32::UI::Accessibility::IUIAutomationElement;
pub struct WindowsProfilesWindow<'a> {
    pub(in crate::application::features) driver: &'a UiDriver,
    pub(in crate::application::features) window: IUIAutomationElement,
    pub(in crate::application::features) pid: u32,
}

impl<'a> WindowsProfilesWindow<'a> {
    pub fn close(&self) -> Result<()> {
        self.driver.close_window(&self.window)?;
        self.driver.wait_window_absent(self.pid, "Keyringd")
    }

    pub fn capture_screenshot(&self, name: &str) -> anyhow::Result<()> {
        keyringd_testkit::record_image(name, self.screenshot()?)
    }
    pub fn screenshot(&self) -> Result<std::path::PathBuf> {
        self.driver.capture(&self.window)
    }

    pub fn open_ssh_client_configuration(&self) -> Result<()> {
        self.driver
            .invoke(&self.window, "profiles.configure-ssh-clients")
    }

    pub fn profile_enabled(&self, name: &str) -> Result<bool> {
        self.driver
            .toggle_state_named(&self.window, &format!("Enable profile {name}"))
    }

    pub fn open_device_key_protection(
        &self,
        name: &str,
        change: keyringd_core::service::DeviceKeyProtectionChange,
    ) -> Result<crate::application::features::profiles::windows::DeviceKeyProtectionWindow<'a>>
    {
        use keyringd_core::service::DeviceKeyProtectionChange;
        let operation = match change {
            DeviceKeyProtectionChange::Enable => "Enable local authentication",
            DeviceKeyProtectionChange::ChangePassword => "Reconfigure local authentication",
            DeviceKeyProtectionChange::Disable => "Disable local authentication",
        };
        self.driver.wait_window_enabled(&self.window, true)?;
        self.driver
            .invoke_named(&self.window, &format!("Profile actions for {name}"))?;
        self.driver
            .press_application_enter(self.pid, &self.window, &format!("{operation}…"))?;
        let window = self
            .driver
            .window_with_text(self.pid, &format!("{operation} for “{name}”?"))?;
        Ok(
            crate::application::features::profiles::windows::DeviceKeyProtectionWindow {
                driver: self.driver,
                window,
                owner: self.window.clone(),
            },
        )
    }

    pub fn open_connection_test(&self, name: &str) -> Result<()> {
        self.select_action(name, "Test SSH connection…")?;
        self.driver.wait_window_enabled(&self.window, false)?;
        self.driver
            .window_named(self.pid, &format!("Test SSH Connection — {name}"))?;
        Ok(())
    }
    pub fn start_profile_creation(&self) -> Result<WindowsProfileSetupWizard<'a>> {
        self.driver
            .wait_enabled(&self.window, "profiles-create", true)?;
        self.driver.invoke(&self.window, "profiles-create")?;
        // The owned window is discoverable before open() finishes wiring its HWND.
        self.driver.wait_window_enabled(&self.window, false)?;
        self.driver.window_named(self.pid, "Add SSH profile")?;
        Ok(WindowsProfileSetupWizard::new(self.driver, self.pid))
    }

    pub fn profile_visible(&self, name: &str) -> Result<bool> {
        Ok(self
            .driver
            .contains_visible_named(&self.window, "Keys available through the Keyringd agent")?
            && self
                .driver
                .contains_visible_named(&self.window, &format!("Profile actions for {name}"))?
            && self.driver.contains_visible_named(&self.window, name)?)
    }

    pub fn reveal_newest_profile(&self, name: &str) -> Result<()> {
        self.driver
            .reveal_named_at_bottom(&self.window, &format!("Profile actions for {name}"))
    }
    pub fn profile_is_above(&self, first: &str, second: &str) -> Result<bool> {
        self.driver.element_is_above(&self.window, first, second)
    }
    pub fn move_up(&self, name: &str) -> Result<()> {
        self.select_action(name, "Move up")
    }

    pub fn move_down(&self, name: &str) -> Result<()> {
        self.select_action(name, "Move down")
    }

    pub fn move_actions_enabled(&self, name: &str) -> Result<(bool, bool)> {
        self.driver.wait_window_enabled(&self.window, true)?;
        self.driver
            .invoke_named(&self.window, &format!("Profile actions for {name}"))?;
        let result = (|| {
            Ok((
                self.driver
                    .application_element_enabled_named(self.pid, "Move up")?,
                self.driver
                    .application_element_enabled_named(self.pid, "Move down")?,
            ))
        })();
        self.driver.dismiss_popup(&self.window)?;
        self.driver
            .wait_application_absent_named(self.pid, "Move up")?;
        result
    }

    pub fn reorder_before(&self, source: &str, target: &str) -> Result<()> {
        self.driver.wait_window_enabled(&self.window, true)?;
        self.driver.drag_before(&self.window, source, target)
    }

    fn select_action(&self, name: &str, action: &str) -> Result<()> {
        self.driver.wait_window_enabled(&self.window, true)?;
        self.driver
            .invoke_named(&self.window, &format!("Profile actions for {name}"))?;
        self.driver.invoke_application_named(self.pid, action)
    }

    pub fn begin_rename(&self, name: &str) -> Result<()> {
        self.select_action(name, "Rename")?;
        self.driver.element(&self.window, "profiles-name-editor")?;
        Ok(())
    }
    pub fn enter_profile_name(&self, _name: &str, new_name: &str) -> Result<()> {
        self.driver
            .set_value(&self.window, "profiles-name-editor", new_name)
    }
    pub fn profile_name_editor_content(&self, _name: &str) -> Result<String> {
        self.driver.text(&self.window, "profiles-name-editor")
    }
    pub fn confirm_rename(&self, _name: &str) -> Result<()> {
        self.driver
            .press_enter(&self.window, "profiles-name-editor")
    }

    pub fn open_removal(&self, name: &str) -> Result<RemoveProfileWindow<'a>> {
        self.select_action(name, "Remove profile…")?;
        self.driver.wait_window_enabled(&self.window, false)?;
        Ok(RemoveProfileWindow {
            driver: self.driver,
            window: self.driver.window_named(self.pid, "Remove SSH profile")?,
            pid: self.pid,
        })
    }

    pub fn open_details(&self, name: &str) -> Result<ProfileDetailsWindow<'a>> {
        self.select_action(name, "Details…")?;
        self.driver.wait_window_enabled(&self.window, false)?;
        let window = self.driver.window_named(self.pid, "Profile Details")?;
        self.driver
            .wait_visible_named(&window, &format!("Details for “{name}”"))?;
        Ok(ProfileDetailsWindow {
            driver: self.driver,
            window,
            pid: self.pid,
            profile_name: name.to_owned(),
        })
    }

    pub fn open_trust_entries(&self, name: &str) -> Result<TrustEntriesWindow<'a>> {
        self.select_action(name, "Public Keys…")?;
        self.driver.wait_window_enabled(&self.window, false)?;
        Ok(TrustEntriesWindow {
            driver: self.driver,
            window: self
                .driver
                .window_named(self.pid, &format!("Public Keys for “{name}”"))?,
        })
    }

    pub fn toggle_profile(&self, name: &str) -> Result<()> {
        self.driver
            .toggle_named(&self.window, &format!("Enable profile {name}"))
    }
}

impl crate::application::features::profiles::ProfilesWindowDriver for WindowsProfilesWindow<'_> {
    fn creation_enabled(&self) -> Result<bool> {
        Ok(unsafe {
            self.driver
                .element(&self.window, "profiles-create")?
                .CurrentIsEnabled()?
                .as_bool()
        })
    }
    fn device_key_warning(&self) -> Result<String> {
        self.driver.wait("hardware key warning", || {
            Ok(self
                .driver
                .visible_texts(&self.window)?
                .into_iter()
                .find(|text| text.starts_with("Hardware-backed Device Key storage")))
        })
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

    fn profile_present(&self, name: &str) -> Result<bool> {
        Ok(self
            .driver
            .contains_visible_named(&self.window, &format!("Profile actions for {name}"))?
            || self.driver.contains_visible_named(&self.window, name)?)
    }
    fn profile_visible(&self, name: &str) -> Result<bool> {
        WindowsProfilesWindow::profile_visible(self, name)
    }
    fn profile_is_above(&self, upper: &str, lower: &str) -> Result<bool> {
        WindowsProfilesWindow::profile_is_above(self, upper, lower)
    }
    fn begin_rename(&self, name: &str) -> Result<()> {
        WindowsProfilesWindow::begin_rename(self, name)
    }
    fn enter_profile_name(&self, name: &str, new_name: &str) -> Result<()> {
        WindowsProfilesWindow::enter_profile_name(self, name, new_name)
    }
    fn profile_name_editor_content(&self, name: &str) -> Result<String> {
        WindowsProfilesWindow::profile_name_editor_content(self, name)
    }
    fn confirm_rename(&self, name: &str) -> Result<()> {
        WindowsProfilesWindow::confirm_rename(self, name)
    }
    fn screenshot(&self) -> Result<std::path::PathBuf> {
        WindowsProfilesWindow::screenshot(self)
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
        WindowsProfilesWindow::reorder_before(self, source, target)
    }

    fn toggle_profile(&self, name: &str) -> Result<()> {
        WindowsProfilesWindow::toggle_profile(self, name)
    }
    fn profile_enabled(&self, name: &str) -> Result<bool> {
        WindowsProfilesWindow::profile_enabled(self, name)
    }
    fn open_details(&mut self, name: &str) -> Result<Self::ProfileDetailsWindow<'_>> {
        WindowsProfilesWindow::open_details(self, name)
    }
    fn open_removal(&mut self, name: &str) -> Result<Self::RemoveProfileWindow<'_>> {
        WindowsProfilesWindow::open_removal(self, name)
    }
    fn open_trust_entries(&mut self, name: &str) -> Result<Self::TrustEntriesWindow<'_>> {
        WindowsProfilesWindow::open_trust_entries(self, name)
    }
}
