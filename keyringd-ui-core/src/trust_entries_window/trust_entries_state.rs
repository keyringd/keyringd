use super::{TrustEntriesCommand, TrustEntriesEvent, TrustEntryKind};
use crate::ComponentModel;
use keyringd_core::model::ssh_trust::{
    SshCertificateAuthorityTrustEntry, SshDeviceKeyTrustEntry, SshPasskeyTrustEntry,
};
use keyringd_core::model::{Profile, ProfileId};

/// The public values currently shown by one profile's Public Keys view.
#[derive(Clone, PartialEq)]
pub struct TrustEntriesState {
    profile_id: ProfileId,
    profile_name: String,
    ca: String,
    device_key: String,
    passkey: String,
    registration_notice: Option<String>,
    closed: bool,
}

impl TrustEntriesState {
    pub fn from_profile(profile: &Profile) -> Self {
        Self {
            profile_id: profile.id().clone(),
            profile_name: profile.display_name().to_owned(),
            ca: SshCertificateAuthorityTrustEntry::from_profile(profile).to_string(),
            device_key: SshDeviceKeyTrustEntry::from_profile(profile).to_string(),
            passkey: SshPasskeyTrustEntry::from_profile(profile).to_string(),
            registration_notice: None,
            closed: false,
        }
    }
    pub fn with_notice(mut self, notice: Option<String>) -> Self {
        self.registration_notice = notice;
        self
    }
    pub fn profile_id(&self) -> &ProfileId {
        &self.profile_id
    }
    pub fn heading(&self) -> String {
        crate::trust_entries_window::heading(&self.profile_name)
    }
    pub fn registration_notice(&self) -> Option<&str> {
        self.registration_notice.as_deref()
    }
    pub fn value(&self, kind: TrustEntryKind) -> String {
        match kind {
            TrustEntryKind::CertificateAuthority => self.ca.clone(),
            TrustEntryKind::DeviceKey => self.device_key.clone(),
            TrustEntryKind::Passkey => self.passkey.clone(),
            TrustEntryKind::DirectKeys => format!("{}\n{}", self.device_key, self.passkey),
        }
    }
}

impl ComponentModel for TrustEntriesState {
    type Event = TrustEntriesEvent;
    type Command = TrustEntriesCommand;

    fn update(&mut self, event: Self::Event) -> Option<Self::Command> {
        if self.closed {
            return None;
        }
        match event {
            TrustEntriesEvent::CopyRequested(kind) => Some(TrustEntriesCommand::Copy {
                value: self.value(kind),
                success_message: kind.success_message(),
            }),
            TrustEntriesEvent::EntriesChanged(mut updated) => {
                if updated.profile_id != self.profile_id {
                    return None;
                }
                updated.registration_notice = self.registration_notice.take();
                *self = updated;
                None
            }
            TrustEntriesEvent::ProfileRemoved | TrustEntriesEvent::CloseRequested => {
                self.closed = true;
                Some(TrustEntriesCommand::Close)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(id: &str, key: &str) -> TrustEntriesState {
        TrustEntriesState {
            profile_id: ProfileId::from_stored(id.into()),
            profile_name: "Work".into(),
            ca: "ca".into(),
            device_key: key.into(),
            passkey: "passkey".into(),
            registration_notice: None,
            closed: false,
        }
    }

    keyringd_testkit::scenario! {
        group "TrustEntriesState::update" {
            case "Each copy request selects the corresponding entry" {
                verify "copy the text for the requested trust entry kind" {
                    let mut state = fixture("a", "device");
                    for (kind, expected) in [
                        (TrustEntryKind::CertificateAuthority, "ca"),
                        (TrustEntryKind::DeviceKey, "device"),
                        (TrustEntryKind::Passkey, "passkey"),
                        (TrustEntryKind::DirectKeys, "device\npasskey"),
                    ] {
                        assert_eq!(
                            state.update(TrustEntriesEvent::CopyRequested(kind)),
                            Some(TrustEntriesCommand::Copy {
                                value: expected.into(),
                                success_message: kind.success_message()
                            })
                        );
                    }
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "TrustEntriesState::update" {
            case "Copying after an update uses current keys and keeps the registration notice" {
                verify "copy the latest key data without discarding the registration notice" {
                    let mut state = fixture("a", "old").with_notice(Some("Update registration".into()));
                    let copy_request = TrustEntriesEvent::CopyRequested(TrustEntryKind::DeviceKey);
                    let mut updated = fixture("a", "new");
                    updated.profile_name = "Renamed".into();
                    state.update(TrustEntriesEvent::EntriesChanged(updated));
                    assert_eq!(state.heading(), "Public Keys for “Renamed”");
                    assert_eq!(state.registration_notice(), Some("Update registration"));
                    assert!(
                        matches!(state.update(copy_request), Some(TrustEntriesCommand::Copy { value, .. }) if value == "new")
                    );
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "TrustEntriesState::update" {
            case "Updates for another profile are ignored" {
                verify "keep the displayed trust entries when a different profile changes" {
                    let mut current = fixture("a", "current");
                    current.update(TrustEntriesEvent::EntriesChanged(fixture("b", "other")));
                    assert_eq!(current.value(TrustEntryKind::DeviceKey), "current");
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "TrustEntriesState::update" {
            case "Closed or removed views reject queued copy requests" {
                verify "suppress clipboard commands after the view closes or loses its profile" {
                    for event in [
                        TrustEntriesEvent::CloseRequested,
                        TrustEntriesEvent::ProfileRemoved,
                    ] {
                        let mut current = fixture("a", "current");
                        assert_eq!(current.update(event), Some(TrustEntriesCommand::Close));
                        assert_eq!(
                            current.update(TrustEntriesEvent::CopyRequested(TrustEntryKind::DirectKeys)),
                            None
                        );
                        assert_eq!(current.update(TrustEntriesEvent::ProfileRemoved), None);
                    }
                }
            }
        }
    }
}
