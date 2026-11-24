use keyringd_core::model::{Profile, ProfileId};

use super::{ProfileDetailsCommand, ProfileDetailsEvent};
use crate::ComponentModel;

#[derive(Clone, PartialEq)]
pub struct ProfileDetailsState {
    profile_id: ProfileId,
    heading: String,
    status: &'static str,
    device_key_signature_algorithm: String,
    device_key_local_authentication: &'static str,
    error: Option<String>,
    closed: bool,
}

impl ProfileDetailsState {
    pub fn new(profile: &Profile) -> Self {
        Self {
            profile_id: profile.id().clone(),
            heading: crate::profile_details_window::heading(profile.display_name()),
            status: crate::profile_details_window::status(profile.is_enabled()),
            device_key_signature_algorithm: profile.device_key().signature_algorithm().to_string(),
            device_key_local_authentication:
                crate::device_key_protection_window::authentication_label(
                    profile.device_key().local_authentication(),
                ),
            error: None,
            closed: false,
        }
    }

    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    pub fn profile_id(&self) -> &ProfileId {
        &self.profile_id
    }

    pub fn heading(&self) -> &str {
        &self.heading
    }

    pub fn status(&self) -> &'static str {
        self.status
    }

    pub fn device_key_signature_algorithm(&self) -> &str {
        &self.device_key_signature_algorithm
    }

    pub fn device_key_local_authentication(&self) -> &'static str {
        self.device_key_local_authentication
    }
}

impl ComponentModel for ProfileDetailsState {
    type Event = ProfileDetailsEvent;
    type Command = ProfileDetailsCommand;

    fn update(&mut self, event: ProfileDetailsEvent) -> Option<ProfileDetailsCommand> {
        if self.closed {
            return None;
        }
        match event {
            ProfileDetailsEvent::TrustEntriesRequested => {
                self.error = None;
                Some(ProfileDetailsCommand::OpenTrustEntries)
            }
            ProfileDetailsEvent::ReplaceDeviceKeyRequested => {
                self.error = None;
                Some(ProfileDetailsCommand::OpenReplaceDeviceKey)
            }
            ProfileDetailsEvent::ProfileChanged(profile) => {
                if profile.id() != &self.profile_id {
                    return None;
                }
                let error = self.error.take();
                *self = Self::new(&profile);
                self.error = error;
                None
            }
            ProfileDetailsEvent::ProfileRemoved | ProfileDetailsEvent::CloseRequested => {
                self.closed = true;
                Some(ProfileDetailsCommand::Close)
            }
            ProfileDetailsEvent::ChildWindowOpened(result) => {
                self.error = result.err();
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state() -> ProfileDetailsState {
        ProfileDetailsState {
            profile_id: ProfileId::from_stored("test".into()),
            heading: "Details".into(),
            status: "Enabled",
            device_key_signature_algorithm: "ecdsa".into(),
            device_key_local_authentication: "Required",
            error: None,
            closed: false,
        }
    }

    keyringd_testkit::scenario! {
        group "ProfileDetailsState::update" {
            case "Removed profiles close once and ignore stale actions" {
                verify "emit one close command when the profile disappears and ignore subsequent actions" {
                    let mut state = state();
                    assert_eq!(
                        state.update(ProfileDetailsEvent::ProfileRemoved),
                        Some(ProfileDetailsCommand::Close)
                    );
                    assert_eq!(state.update(ProfileDetailsEvent::ProfileRemoved), None);
                    assert_eq!(
                        state.update(ProfileDetailsEvent::TrustEntriesRequested),
                        None
                    );
                    assert_eq!(
                        state.update(ProfileDetailsEvent::ReplaceDeviceKeyRequested),
                        None
                    );
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "ProfileDetailsState::update" {
            case "Retrying a child window clears the previous open error" {
                verify "clear the displayed open error before issuing the retry command" {
                    let mut state = state();
                    state.update(ProfileDetailsEvent::ChildWindowOpened(Err(
                        "open failed".into()
                    )));
                    assert_eq!(state.error(), Some("open failed"));
                    assert_eq!(
                        state.update(ProfileDetailsEvent::TrustEntriesRequested),
                        Some(ProfileDetailsCommand::OpenTrustEntries)
                    );
                    assert_eq!(state.error(), None);
                    state.update(ProfileDetailsEvent::ChildWindowOpened(Ok(())));
                    assert_eq!(
                        state.update(ProfileDetailsEvent::ReplaceDeviceKeyRequested),
                        Some(ProfileDetailsCommand::OpenReplaceDeviceKey)
                    );
                }
            }
        }
    }
}
