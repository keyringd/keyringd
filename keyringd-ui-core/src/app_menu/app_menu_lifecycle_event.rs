use super::AppMenuEvent;
use keyringd_core::model::Profile;

pub enum AppMenuLifecycleEvent {
    Mounted(Vec<Profile>),
    ProfilesChanged(Vec<Profile>),
    ActionRequested(AppMenuEvent),
    StopRequested,
}
