use super::ProfileMutationRequest;
use crate::toast::ToastMessage;
use keyringd_core::model::ProfileId;
use keyringd_core::service::DeviceKeyProtectionChange;
pub enum ProfilesWindowCommand {
    CheckDeviceKeyAvailability,
    AddProfile,
    ShowDetails(ProfileId),
    ShowTrustEntries(ProfileId),
    TestConnection(ProfileId),
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
    UpdateProfiles(ProfileMutationRequest),
    ShowToast(ToastMessage),
}
