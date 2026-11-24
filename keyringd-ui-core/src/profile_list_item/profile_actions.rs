use super::{ProfileAction, ProfileListItemEvent};
use crate::profile_list::ProfileMoveDirection;
use crate::text;

use keyringd_core::model::Profile;
use keyringd_core::model::ProfileId;
use keyringd_core::model::device_key::LocalAuthentication;
use keyringd_core::service::DeviceKeyProtectionChange;

pub struct ProfileActions {
    pub items: Vec<ProfileAction>,
    id: ProfileId,
    enabled: bool,
}
impl ProfileActions {
    pub fn for_profile(
        profile: &Profile,
        protection_change_label: &'static str,
        position: (usize, usize),
    ) -> Self {
        Self::new(
            profile.id().clone(),
            profile.display_name().to_owned(),
            profile.is_enabled(),
            profile.device_key().local_authentication(),
            protection_change_label,
            position,
        )
    }
    pub fn new(
        id: ProfileId,
        name: String,
        enabled: bool,
        authentication: LocalAuthentication,
        protection_change_label: &'static str,
        position: (usize, usize),
    ) -> Self {
        let mut items = vec![
            ProfileAction::new(
                text::PROFILE_LIST_ITEM_DETAILS,
                "profile-actions.details",
                true,
                false,
                ProfileListItemEvent::ShowDetails(id.clone()),
            ),
            ProfileAction::new(
                text::TRUST_ENTRIES_MENU_ACTION,
                "profile-actions.trust-entries",
                true,
                false,
                ProfileListItemEvent::ShowTrustEntries(id.clone()),
            ),
            ProfileAction::new(
                text::PROFILE_LIST_ITEM_TEST_CONNECTION,
                "profile-actions.connection-test",
                enabled,
                false,
                ProfileListItemEvent::TestConnection(id.clone()),
            ),
            ProfileAction::new(
                text::PROFILE_LIST_RENAME,
                "profile-actions.rename",
                true,
                false,
                ProfileListItemEvent::BeginRename {
                    id: id.clone(),
                    name: name.clone(),
                },
            ),
        ];
        let changes = match authentication {
            LocalAuthentication::Required => vec![
                (
                    protection_change_label,
                    "profile-actions.reauthorize-device-key",
                    DeviceKeyProtectionChange::ChangePassword,
                ),
                (
                    text::DEVICE_KEY_PROTECTION_DISABLE,
                    "profile-actions.disable-device-key-authorization",
                    DeviceKeyProtectionChange::Disable,
                ),
            ],
            LocalAuthentication::NotRequired => vec![(
                text::DEVICE_KEY_PROTECTION_ENABLE,
                "profile-actions.require-device-key-authorization",
                DeviceKeyProtectionChange::Enable,
            )],
        };
        for (index, (label, identifier, change)) in changes.into_iter().enumerate() {
            items.push(ProfileAction::new(
                label,
                identifier,
                true,
                index == 0,
                ProfileListItemEvent::ChangeDeviceKeyProtection {
                    id: id.clone(),
                    name: name.clone(),
                    change,
                },
            ));
        }
        items.push(ProfileAction::new(
            text::REPLACE_DEVICE_KEY_MENU_ACTION,
            "profile-actions.replace-device-key",
            true,
            false,
            ProfileListItemEvent::ReplaceDeviceKey {
                id: id.clone(),
                name: name.clone(),
            },
        ));

        for (label, identifier, direction, available) in [
            (
                text::PROFILE_LIST_MOVE_UP,
                "profile-actions.move-up",
                ProfileMoveDirection::Up,
                position.0 > 0,
            ),
            (
                text::PROFILE_LIST_MOVE_DOWN,
                "profile-actions.move-down",
                ProfileMoveDirection::Down,
                position.0 + 1 < position.1,
            ),
        ] {
            items.push(ProfileAction::new(
                label,
                identifier,
                available,
                direction == ProfileMoveDirection::Up,
                ProfileListItemEvent::Move {
                    id: id.clone(),
                    direction,
                },
            ));
        }

        items.push(ProfileAction::new(
            text::REMOVE_PROFILE_MENU_ACTION,
            "profile-actions.remove",
            true,
            true,
            ProfileListItemEvent::Remove {
                id: id.clone(),
                name,
            },
        ));
        Self { items, id, enabled }
    }
    pub fn enabled_changed(&self, enabled: bool) -> Option<ProfileListItemEvent> {
        (enabled != self.enabled).then(|| ProfileListItemEvent::SetEnabled {
            id: self.id.clone(),
            enabled,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn actions(
        authentication: LocalAuthentication,
        enabled: bool,
        position: (usize, usize),
        label: &'static str,
    ) -> ProfileActions {
        ProfileActions::new(
            ProfileId::from_stored("profile".into()),
            "Name".into(),
            enabled,
            authentication,
            label,
            position,
        )
    }
    keyringd_testkit::scenario! {
        group "ProfileActions::new" {
            case "Protected key uses backend label and same change operation" {
                verify "render the backend's protection label and dispatch the same change action" {
                    for label in [
                        text::DEVICE_KEY_PROTECTION_CHANGE_PASSWORD,
                        text::DEVICE_KEY_PROTECTION_RECONFIGURE,
                    ] {
                        let actions = actions(LocalAuthentication::Required, true, (0, 1), label);
                        let action = actions
                            .items
                            .iter()
                            .find(|a| a.identifier == "profile-actions.reauthorize-device-key")
                            .unwrap();
                        assert_eq!(action.label, label);
                        assert!(matches!(
                            action.activate(),
                            Some(ProfileListItemEvent::ChangeDeviceKeyProtection {
                                change: DeviceKeyProtectionChange::ChangePassword,
                                ..
                            })
                        ));
                        assert!(
                            actions
                                .items
                                .iter()
                                .any(|a| a.label == text::DEVICE_KEY_PROTECTION_DISABLE)
                        );
                        assert!(
                            !actions
                                .items
                                .iter()
                                .any(|a| a.label == text::DEVICE_KEY_PROTECTION_ENABLE)
                        );
                    }
                }
            }
        }
    }
    keyringd_testkit::scenario! {
        group "ProfileActions::new" {
            case "Disabled profile cannot test and movement respects boundaries" {
                verify "disable connection testing for a disabled profile and enforce list endpoints" {
                    let actions = actions(
                        LocalAuthentication::NotRequired,
                        false,
                        (0, 2),
                        text::DEVICE_KEY_PROTECTION_RECONFIGURE,
                    );
                    assert!(
                        actions
                            .items
                            .iter()
                            .find(|a| a.identifier == "profile-actions.connection-test")
                            .unwrap()
                            .activate()
                            .is_none()
                    );
                    assert!(
                        actions
                            .items
                            .iter()
                            .find(|a| a.identifier == "profile-actions.move-up")
                            .unwrap()
                            .activate()
                            .is_none()
                    );
                    assert!(matches!(
                        actions
                            .items
                            .iter()
                            .find(|a| a.identifier == "profile-actions.move-down")
                            .unwrap()
                            .activate(),
                        Some(ProfileListItemEvent::Move {
                            direction: ProfileMoveDirection::Down,
                            ..
                        })
                    ));
                    assert!(actions.enabled_changed(false).is_none());
                    assert!(matches!(
                        actions.enabled_changed(true),
                        Some(ProfileListItemEvent::SetEnabled { enabled: true, .. })
                    ));
                }
            }
        }
    }
    keyringd_testkit::scenario! {
        group "ProfileActions::new" {
            case "Menu commands keep the profile target" {
                verify "dispatch every menu command with the selected profile ID" {
                    let id = ProfileId::from_stored("target".into());
                    let actions = ProfileActions::new(
                        id.clone(),
                        "Target name".into(),
                        true,
                        LocalAuthentication::NotRequired,
                        text::DEVICE_KEY_PROTECTION_RECONFIGURE,
                        (1, 3),
                    );
                    let action = |identifier| {
                        actions
                            .items
                            .iter()
                            .find(|a| a.identifier == identifier)
                            .unwrap()
                            .activate()
                    };
                    assert!(
                        matches!(action("profile-actions.replace-device-key"), Some(ProfileListItemEvent::ReplaceDeviceKey { id: target, name }) if target == id && name == "Target name")
                    );
                    for (identifier, expected) in [
                        ("profile-actions.move-up", ProfileMoveDirection::Up),
                        ("profile-actions.move-down", ProfileMoveDirection::Down),
                    ] {
                        assert!(
                            matches!(action(identifier), Some(ProfileListItemEvent::Move { id: target, direction }) if target == id && direction == expected)
                        );
                    }
                    assert!(
                        matches!(action("profile-actions.remove"), Some(ProfileListItemEvent::Remove { id: target, name }) if target == id && name == "Target name")
                    );
                }
            }
        }
    }
}
