use super::DeviceKeyProtectionOutcome;

pub enum DeviceKeyProtectionEvent {
    CloseRequested,
    Started,
    Finished(DeviceKeyProtectionOutcome),
}
