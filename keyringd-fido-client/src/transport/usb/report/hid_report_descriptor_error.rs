/// An error that indicates that the HID report descriptor is invalid or that the I/O report to be used for ctaphid cannot be uniquely determined.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum HidReportDescriptorError {
    #[error("HID report descriptor is truncated")]
    Truncated,
    #[error("HID report descriptor has an invalid global-state stack")]
    InvalidGlobalState,
    #[error("HID report descriptor does not define one matching CTAPHID input/output report")]
    MissingOrAmbiguousReport,
    #[error("HID report size is not a whole number of bytes")]
    NonByteAlignedReport,
    #[error("HID report size exceeds the platform address space")]
    ReportTooLarge,
}
