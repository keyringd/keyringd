//! Enumeration and connection of FIDO HID devices by HIDAPI and input/output of HID reports.
//!
//! Since the HIDAPI native management state is shared throughout the process, initialization, enumeration, OPEN, and CLOSE are
//! Consolidate into the surviving management threads until the end of the process. Tokio's temporary runloop on macOS
//! It does not tie to the life of the Blocking Worker. Read/Write is excluded on a per device basis and the last I/O is
//! Return only Close after it's finished to the management thread.
#[cfg(any(target_os = "linux", target_os = "macos"))]
mod hid_api_backend;
#[cfg(any(target_os = "linux", target_os = "macos"))]
mod hid_api_io_error;
#[cfg(any(target_os = "linux", target_os = "macos"))]
mod hid_api_packet_io;
#[cfg(any(target_os = "linux", target_os = "macos"))]
mod hid_api_runtime;
#[cfg(any(target_os = "linux", target_os = "macos"))]
mod hid_api_task;
#[cfg(any(target_os = "linux", target_os = "macos"))]
mod owned_hid_device;

#[cfg(any(target_os = "linux", target_os = "macos"))]
pub use hid_api_backend::HidApiBackend;
#[cfg(any(target_os = "linux", target_os = "macos"))]
pub use hid_api_io_error::HidApiIoError;
#[cfg(any(target_os = "linux", target_os = "macos"))]
pub use hid_api_packet_io::HidApiPacketIo;

mod device_matching;
#[cfg(any(test, target_os = "linux", target_os = "macos"))]
pub(crate) use device_matching::is_ctap_hid_device;

#[cfg(all(test, any(target_os = "linux", target_os = "macos")))]
mod drop_probe;
#[cfg(all(test, any(target_os = "linux", target_os = "macos")))]
use drop_probe::DropProbe;

#[cfg(test)]
pub(crate) use device_matching::FIDO_USAGE_PAGE;

#[cfg(test)]
pub(crate) use device_matching::CTAP_HID_USAGE;
