pub(crate) struct UsbIpTransferResult {
    pub actual_length: usize,
    pub data: Vec<u8>,
}
