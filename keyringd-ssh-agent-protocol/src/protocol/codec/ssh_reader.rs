use std::io;

pub struct SshReader<'a> {
    remaining: &'a [u8],
}

impl<'a> SshReader<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        Self { remaining: bytes }
    }

    pub fn read_u8(&mut self) -> io::Result<u8> {
        let (&value, remaining) = self
            .remaining
            .split_first()
            .ok_or_else(|| io::Error::from(io::ErrorKind::UnexpectedEof))?;
        self.remaining = remaining;
        Ok(value)
    }

    pub fn read_u32(&mut self) -> io::Result<u32> {
        let bytes = self.take(4)?;
        Ok(u32::from_be_bytes(
            bytes.try_into().expect("length checked"),
        ))
    }

    pub fn read_u64(&mut self) -> io::Result<u64> {
        let bytes = self.take(8)?;
        Ok(u64::from_be_bytes(
            bytes.try_into().expect("length checked"),
        ))
    }

    pub fn read_string(&mut self) -> io::Result<&'a [u8]> {
        let length = self.read_u32()? as usize;
        self.take(length)
    }

    pub fn remaining(&self) -> &'a [u8] {
        self.remaining
    }

    pub fn finish(self) -> io::Result<()> {
        if self.remaining.is_empty() {
            Ok(())
        } else {
            Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("{} trailing bytes", self.remaining.len()),
            ))
        }
    }

    fn take(&mut self, length: usize) -> io::Result<&'a [u8]> {
        if self.remaining.len() < length {
            return Err(io::Error::from(io::ErrorKind::UnexpectedEof));
        }
        let (value, remaining) = self.remaining.split_at(length);
        self.remaining = remaining;
        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use super::SshReader;

    keyringd_testkit::scenario! {
        group "SshReader::read_u32" {
            case "Read Integer and String in Wire order" {
                spec "https://www.rfc-editor.org/rfc/rfc4251.html#section-5"
                given "Prepare a Buffer with U32 followed by SSH String" {
                    let bytes = [0, 0, 0, 7, 0, 0, 0, 3, b's', b's', b'h'];
                    let mut reader = SshReader::new(&bytes);
                }
                when "Read values in Wire order" {
                    let integer = reader.read_u32();
                    let string = reader.read_string();
                    let finished = reader.finish();
                }
                then "Decode both values to leave no INPUT" {
                    assert_eq!(integer.unwrap(), 7);
                    assert_eq!(string.unwrap(), b"ssh");
                    assert!(finished.is_ok());
                }
            }
        }
    }
}
