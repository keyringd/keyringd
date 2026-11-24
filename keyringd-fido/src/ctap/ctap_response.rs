use super::CtapStatusCode;
use crate::TransportError;

/// CTAP response status and encoded payload.
/// Read the boundaries of the response in the communication layer, and leave the clients to determine success/failures and interpret CBOR.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CtapResponse {
    status: CtapStatusCode,
    payload: Vec<u8>,
}

impl CtapResponse {
    pub fn new(status: CtapStatusCode, payload: Vec<u8>) -> Self {
        Self { status, payload }
    }
    pub fn from_bytes(encoded: &[u8]) -> Result<Self, TransportError> {
        let (&status, payload) = encoded
            .split_first()
            .ok_or(TransportError::MissingCtapStatus)?;
        Ok(Self::new(
            CtapStatusCode::from_byte(status),
            payload.to_vec(),
        ))
    }
    pub const fn status(&self) -> CtapStatusCode {
        self.status
    }
    pub fn payload(&self) -> &[u8] {
        &self.payload
    }
    pub fn into_payload(self) -> Vec<u8> {
        self.payload
    }
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut encoded = Vec::with_capacity(1 + self.payload.len());
        encoded.push(self.status.as_byte());
        encoded.extend_from_slice(&self.payload);
        encoded
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    keyringd_testkit::scenario! {
        group "CtapResponse::from_bytes" {
            case "Wire response preserves status and payload" {
                verify "parse the leading status byte and leave the following payload intact" {
                    for encoded in [vec![0x00, 0xa0], vec![0x06], vec![0xfe, 0x01, 0x02]] {
                        let response = CtapResponse::from_bytes(&encoded).unwrap();
                        assert_eq!(response.status().as_byte(), encoded[0]);
                        assert_eq!(response.payload(), &encoded[1..]);
                        assert_eq!(response.to_bytes(), encoded);
                    }
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "CtapResponse::from_bytes" {
            case "Wire response requires status byte" {
                verify "reject a response with no leading status byte" {
                    assert_eq!(
                        CtapResponse::from_bytes(&[]),
                        Err(TransportError::MissingCtapStatus)
                    );
                }
            }
        }
    }
}
