use crate::text;

use keyringd_core::model::ProfileId;

#[derive(Clone, Default, PartialEq)]
pub struct ProfileNameEditState {
    editing: Option<(ProfileId, String)>,
}

impl ProfileNameEditState {
    pub fn editing(&self) -> Option<(&ProfileId, &str)> {
        self.editing.as_ref().map(|(id, name)| (id, name.as_str()))
    }
    pub fn begin(&mut self, id: ProfileId, name: String) {
        self.editing = Some((id, name));
    }
    pub fn set_name(&mut self, id: &ProfileId, name: String) {
        if let Some((current_id, current_name)) = &mut self.editing
            && current_id == id
        {
            *current_name = name;
        }
    }
    pub fn cancel(&mut self, id: &ProfileId) {
        if self
            .editing
            .as_ref()
            .is_some_and(|(current, _)| current == id)
        {
            self.editing = None;
        }
    }
    /// A request from another row is stale; a failed save keeps the editor open.
    pub fn save(&self, id: &ProfileId) -> Result<Option<String>, &'static str> {
        let Some((current, name)) = self.editing() else {
            return Ok(None);
        };
        if current != id {
            return Ok(None);
        }
        let name = name.trim();
        if name.is_empty() {
            Err(text::PROFILE_LIST_NAME_REQUIRED)
        } else {
            Ok(Some(name.to_owned()))
        }
    }
    pub fn finished(&mut self, id: &ProfileId, succeeded: bool) {
        if succeeded {
            self.cancel(id);
        }
    }
    pub fn retain_profiles<'a>(&mut self, mut ids: impl Iterator<Item = &'a ProfileId>) {
        if self
            .editing
            .as_ref()
            .is_some_and(|(id, _)| !ids.any(|candidate| candidate == id))
        {
            self.editing = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    keyringd_testkit::scenario! {
        group "ProfileNameEditState::save" {
            case "Validation and save failure preserve the draft until success" {
                verify "retain the edited name after rejected input or a save error and clear it on success" {
                    let id = ProfileId::new();
                    let mut state = ProfileNameEditState::default();
                    state.begin(id.clone(), "  ".into());
                    assert_eq!(state.save(&id), Err(text::PROFILE_LIST_NAME_REQUIRED));
                    state.set_name(&id, " Work ".into());
                    assert_eq!(state.save(&id), Ok(Some("Work".into())));
                    state.finished(&id, false);
                    assert_eq!(state.editing(), Some((&id, " Work ")));
                    state.finished(&id, true);
                    assert!(state.editing().is_none());
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "ProfileNameEditState::begin" {
            case "Stale row events and old completions do not affect a different editor" {
                verify "keep the current profile editor intact when it receives another profile's events" {
                    let old = ProfileId::new();
                    let current = ProfileId::new();
                    let mut state = ProfileNameEditState::default();
                    state.begin(current.clone(), "Current".into());
                    state.set_name(&old, "Stale".into());
                    state.cancel(&old);
                    assert_eq!(state.save(&old), Ok(None));
                    state.finished(&old, true);
                    assert_eq!(state.editing(), Some((&current, "Current")));
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "ProfileNameEditState::retain_profiles" {
            case "Removing the edited profile closes the editor" {
                verify "dismiss the name editor when its profile is removed from the list" {
                    let id = ProfileId::new();
                    let other = ProfileId::new();
                    let mut state = ProfileNameEditState::default();
                    state.begin(id.clone(), "Work".into());
                    state.retain_profiles([&id, &other].into_iter());
                    assert!(state.editing().is_some());
                    state.retain_profiles([&other].into_iter());
                    assert!(state.editing().is_none());
                }
            }
        }
    }
}
