use keyringd_core::model::Profile;

#[derive(Clone)]
pub enum ProfileDetailsEvent {
    TrustEntriesRequested,
    ReplaceDeviceKeyRequested,
    ProfileChanged(Box<Profile>),
    ProfileRemoved,
    ChildWindowOpened(Result<(), String>),
    CloseRequested,
}
