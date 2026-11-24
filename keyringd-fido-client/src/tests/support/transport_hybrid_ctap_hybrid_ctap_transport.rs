pub(crate) fn get_info_bytes() -> Vec<u8> {
    let mut encoded = vec![0xa2, 0x01, 0x81, 0x68];
    encoded.extend_from_slice(b"FIDO_2_1");
    encoded.extend_from_slice(&[0x03, 0x50]);
    encoded.extend_from_slice(&[0; 16]);
    encoded
}
