/// An internal state that holds the bit width, number, and ID of the report that is valid while parsing the HID report descriptor.
#[derive(Clone, Copy, Default)]
pub(super) struct GlobalState {
    pub(super) report_size: u64,
    pub(super) report_count: u64,
    pub(super) report_id: u8,
}
