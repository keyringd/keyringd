use std::path::PathBuf;

use anyhow::Result;

/// Only setup, user actions, and observations; expected results live in the scenario.
pub trait ProfilesWindowDriver: Sized {
    /// Observes the Create Profile button in the empty list.
    fn creation_enabled(&self) -> Result<bool>;
    fn device_key_warning(&self) -> Result<String>;
    fn profile_present(&self, name: &str) -> Result<bool>;
    fn profile_visible(&self, name: &str) -> Result<bool>;
    fn begin_rename(&self, profile_name: &str) -> Result<()>;
    fn enter_profile_name(&self, profile_name: &str, new_name: &str) -> Result<()>;
    fn profile_name_editor_content(&self, profile_name: &str) -> Result<String>;
    fn confirm_rename(&self, profile_name: &str) -> Result<()>;
    fn screenshot(&self) -> Result<PathBuf>;
    fn capture_screenshot(&self, name: &str) -> Result<()> {
        keyringd_testkit::record_image(name, self.screenshot()?)
    }
    type ProfileDetailsWindow<'a>: super::ProfileDetailsWindowDriver
    where
        Self: 'a;
    type RemoveProfileWindow<'a>: super::RemoveProfileWindowDriver
    where
        Self: 'a;
    type TrustEntriesWindow<'a>: super::TrustEntriesWindowDriver
    where
        Self: 'a;
    fn open_details(&mut self, name: &str) -> Result<Self::ProfileDetailsWindow<'_>>;
    fn open_removal(&mut self, name: &str) -> Result<Self::RemoveProfileWindow<'_>>;
    fn open_trust_entries(&mut self, name: &str) -> Result<Self::TrustEntriesWindow<'_>>;
    fn move_profile_up(&self, name: &str) -> Result<()>;
    fn move_profile_down(&self, name: &str) -> Result<()>;
    fn profile_move_actions_enabled(&self, name: &str) -> Result<(bool, bool)>;
    fn reorder_before(&self, source: &str, target: &str) -> Result<()>;
    fn profile_is_above(&self, upper: &str, lower: &str) -> Result<bool>;
    fn toggle_profile(&self, profile_name: &str) -> Result<()>;
    fn profile_enabled(&self, profile_name: &str) -> Result<bool>;
}
