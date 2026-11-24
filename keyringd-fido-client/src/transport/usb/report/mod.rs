//! Analyze the HID report descriptor and determine the report ID and packet length for cTaphiD.
mod global_state;
mod hid_report_descriptor_error;
mod hid_report_layout;
mod report_bits;
mod report_descriptor;

pub use hid_report_descriptor_error::HidReportDescriptorError;
pub use hid_report_layout::HidReportLayout;
pub use report_descriptor::parse_report_layout;
