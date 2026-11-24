use super::DeviceKeyReplacementOutcome;

#[derive(Debug)]
pub enum ReplaceDeviceKeyEvent {
    PrimaryRequested,
    CloseRequested,
    ReplacementFinished(DeviceKeyReplacementOutcome),
}
