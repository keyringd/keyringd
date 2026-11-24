pub struct SshWriter {
    bytes: Vec<u8>,
}

impl SshWriter {
    pub fn new() -> Self {
        Self { bytes: Vec::new() }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            bytes: Vec::with_capacity(capacity),
        }
    }

    pub fn write_u8(&mut self, value: u8) {
        self.bytes.push(value);
    }

    pub fn write_u32(&mut self, value: u32) {
        self.bytes.extend_from_slice(&value.to_be_bytes());
    }

    pub fn write_u64(&mut self, value: u64) {
        self.bytes.extend_from_slice(&value.to_be_bytes());
    }

    pub fn write_string(&mut self, value: &[u8]) {
        self.write_u32(value.len() as u32);
        self.bytes.extend_from_slice(value);
    }

    pub fn write_bytes(&mut self, value: &[u8]) {
        self.bytes.extend_from_slice(value);
    }

    pub fn write_mpint(&mut self, value: &[u8]) {
        const SIGN_BIT: u8 = 0x80;

        let value = value
            .iter()
            .position(|&byte| byte != 0)
            .map_or(&[][..], |start| &value[start..]);
        if value.is_empty() {
            self.write_u32(0);
        } else if value[0] & SIGN_BIT != 0 {
            self.write_u32(value.len() as u32 + 1);
            self.bytes.push(0);
            self.bytes.extend_from_slice(value);
        } else {
            self.write_string(value);
        }
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }

    pub fn frame(payload: &[u8]) -> Vec<u8> {
        let mut writer = Self::with_capacity(4 + payload.len());
        writer.write_u32(payload.len() as u32);
        writer.bytes.extend_from_slice(payload);
        writer.into_bytes()
    }
}

impl Default for SshWriter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::SshWriter;
    use crate::protocol::codec::SshReader;

    keyringd_testkit::scenario! {
        group "SshWriter::write_mpint" {
            case "Write a positive MPINT with required padding" {
                spec "https://www.rfc-editor.org/rfc/rfc4251.html#section-5"
                given "Prepare an Integer with Sign Bit in the leading valid byte" {
                    let value = [0, 0x80, 1];
                    let mut writer = SshWriter::new();
                }
                when "Encode Integer as SSH MPINT" {
                    writer.write_mpint(&value);
                    let bytes = writer.into_bytes();
                    let mut reader = SshReader::new(&bytes);
                    let decoded = reader.read_string();
                    let finished = reader.finish();
                }
                then "Keep ZERO SIGN BYTE and leave DATA at the mpint" {
                    assert_eq!(decoded.unwrap(), &[0, 0x80, 1]);
                    assert!(finished.is_ok());
                }
            }
        }
    }
}
