use std::sync::Arc;

use super::{OsUsbBackend, UsbHidBackend};

/// How to run the USB set on the client. Specify either direct communication by HID or delegate to the OS at the time of building.
pub enum UsbBackend {
    Hid(Arc<dyn UsbHidBackend>),
    Os(Arc<dyn OsUsbBackend>),
}
