/// An internal state that aggregates the number of input/output bits for one report ID while parsing the HID report descriptor.
#[derive(Clone, Copy, Default)]
pub(super) struct ReportBits {
    pub(super) input: u64,
    pub(super) output: u64,
}
