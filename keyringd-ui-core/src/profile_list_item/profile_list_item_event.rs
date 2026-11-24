use crate::profile_list::ProfileMoveDirection;
use keyringd_core::model::ProfileId;
use keyringd_core::service::DeviceKeyProtectionChange;
#[derive(Clone)]
pub enum ProfileListItemEvent {
    ShowDetails(ProfileId),
    ShowTrustEntries(ProfileId),
    TestConnection(ProfileId),
    BeginRename {
        id: ProfileId,
        name: String,
    },
    RenameNameChanged {
        id: ProfileId,
        name: String,
    },
    RenameRequested(ProfileId),
    RenameCancelled(ProfileId),
    ChangeDeviceKeyProtection {
        id: ProfileId,
        name: String,
        change: DeviceKeyProtectionChange,
    },
    ReplaceDeviceKey {
        id: ProfileId,
        name: String,
    },
    Remove {
        id: ProfileId,
        name: String,
    },
    SetEnabled {
        id: ProfileId,
        enabled: bool,
    },
    Reorder(Vec<ProfileId>),
    Move {
        id: ProfileId,
        direction: ProfileMoveDirection,
    },
}
