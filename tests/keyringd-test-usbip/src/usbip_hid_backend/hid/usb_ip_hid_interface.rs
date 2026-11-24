use crate::usbip_hid_backend::UsbIpError;

pub(crate) struct UsbIpHidInterface {
    pub configuration: u8,
    pub number: u8,
    pub report_length: u16,
    pub input_endpoint: u8,
    pub output_endpoint: u8,
    pub input_size: usize,
    pub output_size: usize,
    pub input_interval: u32,
    pub output_interval: u32,
}
impl UsbIpHidInterface {
    pub(crate) fn parse(bytes: &[u8]) -> Result<Self, UsbIpError> {
        if bytes.len() < 9
            || bytes[1] != 2
            || usize::from(u16::from_le_bytes([bytes[2], bytes[3]])) != bytes.len()
        {
            return Err(UsbIpError::Protocol("configuration descriptor length"));
        }
        let mut result = None;
        let mut in_hid = false;
        let mut offset = 0;
        while offset < bytes.len() {
            let length = usize::from(bytes[offset]);
            if length < 2 || offset + length > bytes.len() {
                return Err(UsbIpError::Protocol("truncated USB descriptor"));
            }
            let descriptor = &bytes[offset..offset + length];
            match descriptor[1] {
                4 if length >= 9 => {
                    in_hid = descriptor[5] == 3 && descriptor[3] == 0;
                    if in_hid {
                        if result.is_some() {
                            return Err(UsbIpError::Protocol(
                                "multiple HID interfaces are unsupported",
                            ));
                        }
                        result = Some(Self {
                            configuration: bytes[5],
                            number: descriptor[2],
                            report_length: 0,
                            input_endpoint: 0,
                            output_endpoint: 0,
                            input_size: 0,
                            output_size: 0,
                            input_interval: 0,
                            output_interval: 0,
                        });
                    }
                }
                0x21 if in_hid && length >= 9 => {
                    let interface = result.as_mut().unwrap();
                    for item in descriptor[6..].as_chunks::<3>().0 {
                        if item[0] == 0x22 {
                            interface.report_length = u16::from_le_bytes([item[1], item[2]]);
                        }
                    }
                }
                5 if in_hid && length >= 7 && descriptor[3] & 3 == 3 => {
                    let interface = result.as_mut().unwrap();
                    let endpoint = descriptor[2];
                    let size =
                        usize::from(u16::from_le_bytes([descriptor[4], descriptor[5]]) & 0x07ff);
                    if endpoint & 0x80 != 0 {
                        interface.input_endpoint = endpoint & 0x0f;
                        interface.input_size = size;
                        interface.input_interval = u32::from(descriptor[6]);
                    } else {
                        interface.output_endpoint = endpoint & 0x0f;
                        interface.output_size = size;
                        interface.output_interval = u32::from(descriptor[6]);
                    }
                }
                _ => {}
            }
            offset += length;
        }
        result
            .filter(|i| {
                i.report_length > 0
                    && i.input_endpoint > 0
                    && i.output_endpoint > 0
                    && i.input_size > 0
                    && i.output_size > 0
            })
            .ok_or(UsbIpError::Protocol(
                "missing HID report or interrupt endpoints",
            ))
    }
}
