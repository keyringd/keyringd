/// HID report ID and I/O packet length for ctaphid.
/// Analyze and determine the report descriptor and use it for the actual packet I/O.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HidReportLayout {
    pub(crate) report_id: Option<u8>,
    pub(crate) input_packet_size: usize,
    pub(crate) output_packet_size: usize,
}

impl HidReportLayout {
    pub fn report_id(self) -> u8 {
        self.report_id.unwrap_or(0)
    }

    pub const fn uses_report_ids(self) -> bool {
        self.report_id.is_some()
    }

    pub const fn input_packet_size(self) -> usize {
        self.input_packet_size
    }

    pub const fn output_packet_size(self) -> usize {
        self.output_packet_size
    }
}
