#[derive(Debug, thiserror::Error)]
pub enum UsbIpError {
    #[error("USB/IP I/O failed")]
    Io(#[from] std::io::Error),
    #[error("invalid or unsupported USB/IP message: {0}")]
    Protocol(&'static str),
    #[error("USB/IP request failed with status {0}")]
    Status(i32),
    #[error("USB/IP transfer timed out")]
    TimedOut,
    #[error("USB/IP device is not exported by the configured servers")]
    DeviceNotFound,
    #[error("invalid FIDO HID report descriptor")]
    HidDescriptor(#[from] keyringd_fido_client::usb::HidReportDescriptorError),
}

impl UsbIpError {
    pub(crate) fn into_hid_error(self) -> keyringd_fido_client::usb::UsbHidError {
        let disconnected = match &self {
            Self::Io(error) => matches!(
                error.kind(),
                std::io::ErrorKind::ConnectionAborted
                    | std::io::ErrorKind::ConnectionReset
                    | std::io::ErrorKind::BrokenPipe
                    | std::io::ErrorKind::NotConnected
                    | std::io::ErrorKind::UnexpectedEof
            ),
            Self::DeviceNotFound => true,
            _ => false,
        };
        if disconnected {
            keyringd_fido_client::usb::UsbHidError::disconnected(self)
        } else {
            keyringd_fido_client::usb::UsbHidError::new(self)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    keyringd_testkit::scenario! {
        group "UsbIpError::into_hid_error" {
            case "Connection loss is distinguished from timeout protocol and permission failures" {
                verify "only lost connections report disconnection and preserve the original cause" {
                    use std::io::ErrorKind;
                    for (error, disconnected) in [
                        (UsbIpError::Io(std::io::Error::from(ErrorKind::ConnectionAborted)), true),
                        (UsbIpError::Io(std::io::Error::from(ErrorKind::ConnectionReset)), true),
                        (UsbIpError::Io(std::io::Error::from(ErrorKind::BrokenPipe)), true),
                        (UsbIpError::Io(std::io::Error::from(ErrorKind::NotConnected)), true),
                        (UsbIpError::Io(std::io::Error::from(ErrorKind::UnexpectedEof)), true),
                        (UsbIpError::DeviceNotFound, true),
                        (UsbIpError::TimedOut, false),
                        (UsbIpError::Protocol("device disconnected"), false),
                        (UsbIpError::Io(std::io::Error::from(ErrorKind::PermissionDenied)), false),
                        (UsbIpError::Io(std::io::Error::from(ErrorKind::TimedOut)), false),
                    ] {
                        let error = error.into_hid_error();
                        assert_eq!(error.is_disconnected(), disconnected);
                        assert!(std::error::Error::source(&error).unwrap().is::<UsbIpError>());
                    }
                }
            }
        }
    }
}
