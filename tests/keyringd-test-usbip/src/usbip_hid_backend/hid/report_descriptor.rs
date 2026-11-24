pub(crate) fn is_fido_report(bytes: &[u8]) -> bool {
    let mut offset = 0;
    let mut page = 0u32;
    let mut usage = 0u32;
    while offset < bytes.len() {
        let prefix = bytes[offset];
        offset += 1;
        if prefix == 0xfe {
            return false;
        }
        let length = match prefix & 3 {
            3 => 4,
            n => usize::from(n),
        };
        if offset + length > bytes.len() {
            return false;
        }
        let mut value = 0u32;
        for (index, byte) in bytes[offset..offset + length].iter().enumerate() {
            value |= u32::from(*byte) << (index * 8);
        }
        offset += length;
        match ((prefix >> 2) & 3, prefix >> 4) {
            (1, 0) => page = value,
            (2, 0) => usage = value,
            (0, 10) if value == 1 && page == 0xf1d0 && usage == 1 => return true,
            (0, _) => usage = 0,
            _ => {}
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    keyringd_testkit::scenario! {
        group "is_fido_report" {
            case "Non fido and truncated report descriptors are rejected" {
                verify "reject incomplete and non-FIDO HID descriptors" {
                    let fido = [0x06, 0xd0, 0xf1, 0x09, 0x01, 0xa1, 0x01];
                    assert!(is_fido_report(&fido));
                    for length in 0..fido.len() {
                        assert!(!is_fido_report(&fido[..length]));
                    }
                    assert!(!is_fido_report(&[0x05, 0x01, 0x09, 0x06, 0xa1, 0x01]));
                }
            }
        }
    }
}
