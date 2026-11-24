pub(crate) fn descriptor(
    input_report_count: u8,
    output_report_count: u8,
    report_id: Option<u8>,
) -> Vec<u8> {
    let mut value = vec![0x06, 0xd0, 0xf1, 0x09, 0x01, 0xa1, 0x01];
    if let Some(report_id) = report_id {
        value.extend_from_slice(&[0x85, report_id]);
    }
    value.extend_from_slice(&[
        0x09,
        0x20,
        0x15,
        0x00,
        0x26,
        0xff,
        0x00,
        0x75,
        0x08,
        0x95,
        input_report_count,
        0x81,
        0x02,
        0x09,
        0x21,
        0x15,
        0x00,
        0x26,
        0xff,
        0x00,
        0x75,
        0x08,
        0x95,
        output_report_count,
        0x91,
        0x02,
        0xc0,
    ]);
    value
}
