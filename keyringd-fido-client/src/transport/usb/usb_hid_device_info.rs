use super::UsbHidDeviceId;
/// USB backend enumeration results. Summarize the device ID for the backend and the display information of the product/manufacturer.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UsbHidDeviceInfo {
    pub id: UsbHidDeviceId,
    pub vendor_id: u16,
    pub product_id: u16,
    pub manufacturer: Option<String>,
    pub product: Option<String>,
    pub serial_number: Option<String>,
}
