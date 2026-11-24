use super::DeviceKeyAvailabilityError;
use crate::profile_list::ProfileListEvent;
use crate::profile_list_item::ProfileListItemEvent;
use crate::toast::ToastMessage;
use keyringd_core::model::Profile;
#[derive(Clone)]
pub enum ProfilesWindowEvent {
    Opened,
    DeviceKeyAvailabilityChecked(Result<(), DeviceKeyAvailabilityError>),
    AddRequested,
    MutationFinished {
        request_id: u64,
        result: Result<(), String>,
    },
    ProfilesChanged(Vec<Profile>),
    ProfileAction(Box<ProfileListItemEvent>),
    OperationFinished(Result<(), String>),
    ToastRequested(ToastMessage),
}
impl From<ProfileListEvent> for ProfilesWindowEvent {
    fn from(event: ProfileListEvent) -> Self {
        match event {
            ProfileListEvent::AddRequested => Self::AddRequested,
            ProfileListEvent::ProfileAction(action) => Self::ProfileAction(action),
        }
    }
}
