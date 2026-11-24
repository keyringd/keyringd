pub(crate) const FIDO_USAGE_PAGE: u16 = 0xf1d0;
pub(crate) const CTAP_HID_USAGE: u16 = 0x01;

pub(crate) fn is_ctap_hid_device(usage_page: u16, usage: u16) -> bool {
    usage_page == FIDO_USAGE_PAGE && usage == CTAP_HID_USAGE
}
