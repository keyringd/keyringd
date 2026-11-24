pub(crate) const MAX_TRANSFER: usize = 65536;
pub(crate) fn transfer_header(
    command: u32,
    sequence: u32,
    device: u32,
    input: bool,
    endpoint: u8,
) -> [u8; 48] {
    let mut b = [0; 48];
    for (offset, value) in [
        (0, command),
        (4, sequence),
        (8, device),
        (12, u32::from(input)),
        (16, u32::from(endpoint)),
    ] {
        b[offset..offset + 4].copy_from_slice(&value.to_be_bytes());
    }
    b
}
