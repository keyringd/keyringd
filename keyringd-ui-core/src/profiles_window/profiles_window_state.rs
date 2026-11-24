use super::{
    DeviceKeyAvailabilityError, ProfileMutation, ProfileMutationRequest, ProfileNameEditState,
    ProfilesWindowCommand, ProfilesWindowEvent,
};
use crate::profile_list::ProfileMoveDirection;
use crate::profile_list_item::ProfileListItemEvent;
use crate::{ComponentModel, toast::ToastMessage};
use keyringd_core::model::{Profile, ProfileId};
#[derive(Clone, Default)]
pub struct ProfilesWindowState {
    profiles: Vec<Profile>,
    device_key_availability: Option<Result<(), DeviceKeyAvailabilityError>>,
    rename: ProfileNameEditState,
    revision: u64,
    error: Option<String>,
    pending: Option<ProfileMutationRequest>,
    next_request_id: u64,
}

impl PartialEq for ProfilesWindowState {
    fn eq(&self, other: &Self) -> bool {
        self.profiles.len() == other.profiles.len()
            && self.profiles.iter().zip(&other.profiles).all(|(a, b)| {
                a.id() == b.id()
                    && a.display_name() == b.display_name()
                    && a.is_enabled() == b.is_enabled()
                    && a.device_key().local_authentication()
                        == b.device_key().local_authentication()
                    && a.device_key().signature_algorithm() == b.device_key().signature_algorithm()
                    && a.passkey().signature_algorithm() == b.passkey().signature_algorithm()
            })
            && self.device_key_availability == other.device_key_availability
            && self.revision == other.revision
            && self.rename == other.rename
            && self.error == other.error
            && self.pending == other.pending
            && self.next_request_id == other.next_request_id
    }
}
impl ProfilesWindowState {
    pub fn new(profiles: Vec<Profile>) -> Self {
        Self {
            profiles,
            ..Self::default()
        }
    }
    pub fn can_add_profile(&self) -> bool {
        matches!(self.device_key_availability, Some(Ok(()))) && !self.is_saving()
    }
    pub fn device_key_error(&self) -> Option<&str> {
        self.device_key_availability
            .as_ref()?
            .as_ref()
            .err()
            .map(DeviceKeyAvailabilityError::message)
    }
    pub fn can_grant_device_key_access(&self) -> bool {
        matches!(
            self.device_key_availability,
            Some(Err(DeviceKeyAvailabilityError::AccessDenied {
                can_grant: true,
                ..
            }))
        )
    }
    pub fn is_saving(&self) -> bool {
        self.pending.is_some()
    }
    fn start_mutation(&mut self, mutation: ProfileMutation) -> Option<ProfilesWindowCommand> {
        self.next_request_id = self.next_request_id.wrapping_add(1);
        let request = ProfileMutationRequest {
            id: self.next_request_id,
            mutation,
        };
        self.pending = Some(request.clone());
        self.error = None;
        self.revision = self.revision.wrapping_add(1);
        Some(ProfilesWindowCommand::UpdateProfiles(request))
    }
    pub fn profiles(&self) -> &[Profile] {
        &self.profiles
    }
    pub fn profiles_revision(&self) -> u64 {
        self.revision
    }
    pub fn editing(&self) -> Option<(&ProfileId, &str)> {
        self.rename.editing()
    }
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }
    fn replace_profiles(&mut self, profiles: Vec<Profile>) {
        self.profiles = profiles;
        self.revision = self.revision.wrapping_add(1);
        self.rename
            .retain_profiles(self.profiles.iter().map(Profile::id));
    }
}
impl ComponentModel for ProfilesWindowState {
    type Event = ProfilesWindowEvent;
    type Command = ProfilesWindowCommand;
    fn update(&mut self, event: Self::Event) -> Option<Self::Command> {
        match event {
            ProfilesWindowEvent::Opened => {
                self.device_key_availability = None;
                Some(ProfilesWindowCommand::CheckDeviceKeyAvailability)
            }
            ProfilesWindowEvent::DeviceKeyAvailabilityChecked(result) => {
                self.device_key_availability = Some(result);
                None
            }
            ProfilesWindowEvent::AddRequested if !self.can_add_profile() => None,
            ProfilesWindowEvent::ProfileAction(_) | ProfilesWindowEvent::AddRequested
                if self.is_saving() =>
            {
                None
            }
            ProfilesWindowEvent::AddRequested => Some(ProfilesWindowCommand::AddProfile),
            ProfilesWindowEvent::ProfilesChanged(profiles) => {
                self.replace_profiles(profiles);
                None
            }
            ProfilesWindowEvent::ProfileAction(action) => match *action {
                ProfileListItemEvent::BeginRename { id, name } => {
                    self.rename.begin(id, name);
                    self.error = None;
                    None
                }
                ProfileListItemEvent::RenameNameChanged { id, name } => {
                    self.rename.set_name(&id, name);
                    None
                }
                ProfileListItemEvent::RenameCancelled(id) => {
                    self.rename.cancel(&id);
                    None
                }
                ProfileListItemEvent::RenameRequested(id) => match self.rename.save(&id) {
                    Ok(Some(name)) => self.start_mutation(ProfileMutation::Rename { id, name }),
                    Ok(None) => None,
                    Err(error) => {
                        self.error = Some(error.into());
                        Some(ProfilesWindowCommand::ShowToast(ToastMessage::Error(
                            error.into(),
                        )))
                    }
                },
                ProfileListItemEvent::ShowDetails(id) => {
                    Some(ProfilesWindowCommand::ShowDetails(id))
                }
                ProfileListItemEvent::ShowTrustEntries(id) => {
                    Some(ProfilesWindowCommand::ShowTrustEntries(id))
                }
                ProfileListItemEvent::TestConnection(id) => {
                    Some(ProfilesWindowCommand::TestConnection(id))
                }
                ProfileListItemEvent::ChangeDeviceKeyProtection { id, name, change } => {
                    Some(ProfilesWindowCommand::ChangeDeviceKeyProtection { id, name, change })
                }
                ProfileListItemEvent::ReplaceDeviceKey { id, name } => {
                    Some(ProfilesWindowCommand::ReplaceDeviceKey { id, name })
                }
                ProfileListItemEvent::Remove { id, name } => {
                    Some(ProfilesWindowCommand::Remove { id, name })
                }
                ProfileListItemEvent::SetEnabled { id, enabled } => {
                    if self
                        .profiles
                        .iter()
                        .any(|profile| profile.id() == &id && profile.is_enabled() == enabled)
                    {
                        return None;
                    }
                    self.start_mutation(ProfileMutation::SetEnabled { id, enabled })
                }
                ProfileListItemEvent::Reorder(ids) => {
                    if self.profiles.iter().map(Profile::id).eq(ids.iter()) {
                        return None;
                    }
                    self.start_mutation(ProfileMutation::Reorder(ids))
                }
                ProfileListItemEvent::Move { id, direction } => {
                    let mut ids: Vec<_> = self.profiles.iter().map(|p| p.id().clone()).collect();
                    let index = ids.iter().position(|value| value == &id)?;
                    let target = match direction {
                        ProfileMoveDirection::Up => index.checked_sub(1)?,
                        ProfileMoveDirection::Down => {
                            (index + 1 < ids.len()).then_some(index + 1)?
                        }
                    };
                    ids.swap(index, target);
                    self.start_mutation(ProfileMutation::Reorder(ids))
                }
            },
            ProfilesWindowEvent::MutationFinished { request_id, result } => {
                if self
                    .pending
                    .as_ref()
                    .is_none_or(|request| request.id != request_id)
                {
                    return None;
                }
                let request = self
                    .pending
                    .take()
                    .expect("the matching request is pending");
                if let ProfileMutation::Rename { id, .. } = request.mutation {
                    self.rename.finished(&id, result.is_ok());
                }
                // Reconcile native switches/reordering even on failure. Profile snapshots
                // arrive only through the watch subscription, never an older completion.
                self.revision = self.revision.wrapping_add(1);
                self.error = result.as_ref().err().cloned();
                result
                    .err()
                    .map(|error| ProfilesWindowCommand::ShowToast(ToastMessage::Error(error)))
            }
            ProfilesWindowEvent::OperationFinished(result) => {
                self.error = result.err();
                None
            }
            ProfilesWindowEvent::ToastRequested(toast) => {
                Some(ProfilesWindowCommand::ShowToast(toast))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn action(
        state: &mut ProfilesWindowState,
        action: ProfileListItemEvent,
    ) -> Option<ProfilesWindowCommand> {
        state.update(ProfilesWindowEvent::ProfileAction(Box::new(action)))
    }

    fn rename(
        state: &mut ProfilesWindowState,
        id: &ProfileId,
        name: &str,
    ) -> ProfileMutationRequest {
        action(
            state,
            ProfileListItemEvent::BeginRename {
                id: id.clone(),
                name: name.into(),
            },
        );
        let Some(ProfilesWindowCommand::UpdateProfiles(request)) =
            action(state, ProfileListItemEvent::RenameRequested(id.clone()))
        else {
            panic!("a valid rename must start a mutation");
        };
        request
    }

    keyringd_testkit::scenario! {
        group "ProfilesWindowState::update" {
            case "Saving blocks duplicate submissions and changes to the active draft" {
                verify "lock editing and reject another submission while a save is pending" {
                    let id = ProfileId::new();
                    let mut state = ProfilesWindowState::default();
                    let request = rename(&mut state, &id, " Work ");
                    assert!(matches!(request.mutation, ProfileMutation::Rename { ref name, .. } if name == "Work"));
                    assert!(state.is_saving());
                    for event in [
                        ProfileListItemEvent::RenameRequested(id.clone()),
                        ProfileListItemEvent::SetEnabled {
                            id: id.clone(),
                            enabled: false,
                        },
                        ProfileListItemEvent::Reorder(vec![id.clone()]),
                        ProfileListItemEvent::Move {
                            id: id.clone(),
                            direction: ProfileMoveDirection::Down,
                        },
                        ProfileListItemEvent::RenameNameChanged {
                            id: id.clone(),
                            name: "Lost edit".into(),
                        },
                        ProfileListItemEvent::RenameCancelled(id.clone()),
                        ProfileListItemEvent::BeginRename {
                            id: ProfileId::new(),
                            name: "Other".into(),
                        },
                    ] {
                        assert!(action(&mut state, event).is_none());
                    }
                    assert!(state.update(ProfilesWindowEvent::AddRequested).is_none());
                    assert_eq!(state.editing(), Some((&id, " Work ")));
                    state.update(ProfilesWindowEvent::MutationFinished {
                        request_id: request.id,
                        result: Ok(()),
                    });
                    assert!(!state.is_saving());
                    assert!(state.editing().is_none());
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "ProfilesWindowState::update" {
            case "Failed save keeps the draft and old completions cannot finish a retry" {
                verify "retain the draft on error and apply only the completion for the retried save" {
                    let id = ProfileId::new();
                    let mut state = ProfilesWindowState::default();
                    let first = rename(&mut state, &id, " Work ");
                    assert!(matches!(
                        state.update(ProfilesWindowEvent::MutationFinished {
                            request_id: first.id,
                            result: Err("disk full".into()),
                        }),
                        Some(ProfilesWindowCommand::ShowToast(ToastMessage::Error(_)))
                    ));
                    assert!(!state.is_saving());
                    assert_eq!(state.editing(), Some((&id, " Work ")));
                    let Some(ProfilesWindowCommand::UpdateProfiles(retry)) = action(
                        &mut state,
                        ProfileListItemEvent::RenameRequested(id.clone()),
                    ) else {
                        panic!("retry");
                    };
                    assert_ne!(first.id, retry.id);
                    assert!(
                        state
                            .update(ProfilesWindowEvent::MutationFinished {
                                request_id: first.id,
                                result: Ok(())
                            })
                            .is_none()
                    );
                    assert!(state.is_saving());
                    assert_eq!(state.editing(), Some((&id, " Work ")));
                    state.update(ProfilesWindowEvent::MutationFinished {
                        request_id: retry.id,
                        result: Ok(()),
                    });
                    let other = ProfileId::new();
                    rename(&mut state, &other, "Other");
                    state.update(ProfilesWindowEvent::MutationFinished {
                        request_id: retry.id,
                        result: Err("late failure".into()),
                    });
                    assert!(state.is_saving());
                    assert_eq!(state.editing(), Some((&other, "Other")));
                    assert!(state.error().is_none());
                }
            }
        }
    }
    keyringd_testkit::scenario! {
        group "ProfilesWindowState hardware availability" {
            case "Checks on opening and gates creation while preserving the hardware warning" {
                verify "only an available result enables creation and opening checks again" {
                    let mut state = ProfilesWindowState::default();
                    assert!(!state.can_add_profile());
                    assert!(matches!(state.update(ProfilesWindowEvent::Opened), Some(ProfilesWindowCommand::CheckDeviceKeyAvailability)));
                    assert!(state.update(ProfilesWindowEvent::AddRequested).is_none());
                    state.update(ProfilesWindowEvent::DeviceKeyAvailabilityChecked(Err(
                        DeviceKeyAvailabilityError::Unavailable("Hardware unavailable".into()),
                    )));
                    assert!(!state.can_add_profile());
                    assert!(state.update(ProfilesWindowEvent::AddRequested).is_none());
                    state.update(ProfilesWindowEvent::OperationFinished(Ok(())));
                    state.update(ProfilesWindowEvent::ProfilesChanged(vec![]));
                    assert_eq!(state.device_key_error(), Some("Hardware unavailable"));
                    assert!(matches!(state.update(ProfilesWindowEvent::Opened), Some(ProfilesWindowCommand::CheckDeviceKeyAvailability)));
                    assert!(state.device_key_error().is_none());
                    assert!(!state.can_add_profile());
                    state.update(ProfilesWindowEvent::DeviceKeyAvailabilityChecked(Ok(())));
                    assert!(state.can_add_profile());
                    assert!(matches!(state.update(ProfilesWindowEvent::AddRequested), Some(ProfilesWindowCommand::AddProfile)));
                }
            }
        }
    }
}
