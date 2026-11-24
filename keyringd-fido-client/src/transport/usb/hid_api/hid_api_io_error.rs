/// Errors caused by hiDAPI enumeration, connection, input/output and validation of report descriptors.
#[derive(Debug, thiserror::Error)]
pub enum HidApiIoError {
    #[error("failed to start HID manager thread")]
    StartRuntime(#[source] std::sync::Arc<std::io::Error>),
    #[error("HID manager thread stopped")]
    RuntimeStopped,
    #[error("HID management operation did not complete")]
    ManagementTaskAborted,
    #[error("failed to initialize hidapi")]
    Initialize(#[source] hidapi::HidError),
    #[error("failed to refresh HID device enumeration")]
    Enumerate(#[source] hidapi::HidError),
    #[error("failed to open HID device")]
    Open(#[source] hidapi::HidError),
    #[error("failed to read HID report descriptor")]
    ReadReportDescriptor(#[source] hidapi::HidError),
    #[error("invalid HID report descriptor")]
    InvalidReportDescriptor(#[source] super::super::report::HidReportDescriptorError),
    #[error("failed to write HID report")]
    Write(#[source] hidapi::HidError),
    #[error("HID write returned {actual} bytes; expected {expected}")]
    ShortWrite { expected: usize, actual: usize },
    #[error("failed to read HID report")]
    Read(#[source] hidapi::HidError),
    #[error("HID read timed out")]
    ReadTimedOut,
    #[error("HID read returned {actual} bytes; expected {expected}")]
    InvalidReportLength { expected: usize, actual: usize },
    #[error("HID input report used ID {actual}; expected {expected}")]
    InvalidReportId { expected: u8, actual: u8 },
    #[error("HID device lock was poisoned")]
    LockPoisoned,
    #[error("HID blocking task failed")]
    Task(#[source] tokio::task::JoinError),
}

impl HidApiIoError {
    pub(super) fn into_usb_error(self, device_missing: bool) -> crate::usb::UsbHidError {
        if device_missing && matches!(self, Self::Read(_) | Self::Write(_)) {
            crate::usb::UsbHidError::disconnected(self)
        } else {
            crate::usb::UsbHidError::new(self)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    keyringd_testkit::scenario! {
        group "HidApiIoError::into_usb_error" {
            case "Only failed device IO with confirmed removal is disconnected" {
                verify "messages alone do not classify a failure as disconnection" {
                    for missing in [false, true] {
                        for write in [false, true] {
                            let cause = hidapi::HidError::HidApiError { message: "native IO failed".into() };
                            let error = if write { HidApiIoError::Write(cause) } else { HidApiIoError::Read(cause) };
                            let error = error.into_usb_error(missing);
                            assert_eq!(error.is_disconnected(), missing);
                            assert!(std::error::Error::source(&error).unwrap().is::<HidApiIoError>());
                        }
                        for error in [HidApiIoError::ReadTimedOut, HidApiIoError::InvalidReportLength { expected: 64, actual: 1 }, HidApiIoError::LockPoisoned] {
                            assert!(!error.into_usb_error(missing).is_disconnected());
                        }
                    }
                }
            }
        }
    }
}
