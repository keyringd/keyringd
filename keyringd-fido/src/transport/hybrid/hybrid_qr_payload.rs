use crate::codec::{CborItem as Value, CborSimpleValue};

use super::HybridOperationHint;
use super::codec::{boolean, bytes, decode_canonical_map, encode_map, text, unsigned};
use crate::ProtocolError;

#[derive(Clone, Eq, PartialEq)]
pub struct HybridQrPayload {
    pub(crate) public_key: [u8; 33],
    pub(crate) qr_secret: [u8; 16],
    pub(crate) known_tunnel_server_domains: u64,
    pub(crate) current_time: Option<u64>,
    pub(crate) can_use_state_assisted_transactions: Option<bool>,
    pub(crate) operation_hint: HybridOperationHint,
}

impl std::fmt::Debug for HybridQrPayload {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("HybridQrPayload")
            .field("public_key", &self.public_key)
            .field("qr_secret", &"[REDACTED]")
            .field(
                "known_tunnel_server_domains",
                &self.known_tunnel_server_domains,
            )
            .field("current_time", &self.current_time)
            .field(
                "can_use_state_assisted_transactions",
                &self.can_use_state_assisted_transactions,
            )
            .field("operation_hint", &self.operation_hint)
            .finish()
    }
}

impl HybridQrPayload {
    /// Encodes this payload as the numeric URI displayed in a Hybrid QR code.
    pub fn to_uri(&self) -> Result<String, ProtocolError> {
        let encoded = self.to_cbor_bytes()?;
        let mut uri = String::from("FIDO:/");
        for bytes in encoded.chunks(7) {
            let mut chunk = [0u8; 8];
            chunk[..bytes.len()].copy_from_slice(bytes);
            let value = u64::from_le_bytes(chunk);
            let digits = if bytes.len() == 7 {
                17
            } else {
                (0x0fda8530 >> (4 * bytes.len())) & 0x0f
            };
            uri.push_str(&format!("{value:0digits$}"));
        }
        Ok(uri)
    }

    pub fn new(
        public_key: [u8; 33],
        qr_secret: [u8; 16],
        known_tunnel_server_domains: u64,
        operation_hint: HybridOperationHint,
    ) -> Self {
        Self {
            public_key,
            qr_secret,
            known_tunnel_server_domains,
            current_time: None,
            can_use_state_assisted_transactions: None,
            operation_hint,
        }
    }
    pub fn with_current_time(mut self, value: u64) -> Self {
        self.current_time = Some(value);
        self
    }
    pub const fn public_key(&self) -> &[u8; 33] {
        &self.public_key
    }
    pub const fn operation_hint(&self) -> HybridOperationHint {
        self.operation_hint
    }
    pub fn from_cbor_bytes(encoded: &[u8]) -> Result<Self, ProtocolError> {
        let mut fields = decode_canonical_map(encoded, "hybrid QR payload")?;
        let public_key = fixed_bytes::<33>(
            fields
                .remove(&0)
                .ok_or(ProtocolError::MissingCtapField(0))?,
            "hybrid QR public key",
        )?;
        let qr_secret = fixed_bytes::<16>(
            fields
                .remove(&1)
                .ok_or(ProtocolError::MissingCtapField(1))?,
            "hybrid QR secret",
        )?;
        let known_tunnel_server_domains = unsigned(
            fields
                .remove(&2)
                .ok_or(ProtocolError::MissingCtapField(2))?,
            "hybrid QR known tunnel server domains",
        )?;
        let current_time = fields
            .remove(&3)
            .map(|value| unsigned(value, "hybrid QR current time"))
            .transpose()?;
        let can_use_state_assisted_transactions = fields
            .remove(&4)
            .map(|value| boolean(value, "hybrid QR state-assisted flag"))
            .transpose()?;
        let operation_hint = HybridOperationHint::from_wire_value(&text(
            fields
                .remove(&5)
                .ok_or(ProtocolError::MissingCtapField(5))?,
            "hybrid QR operation hint",
        )?);
        Ok(Self {
            public_key,
            qr_secret,
            known_tunnel_server_domains,
            current_time,
            can_use_state_assisted_transactions,
            operation_hint,
        })
    }

    pub fn to_cbor_bytes(&self) -> Result<Vec<u8>, ProtocolError> {
        let mut fields = vec![
            (0, Value::ByteString(self.public_key.to_vec())),
            (1, Value::ByteString(self.qr_secret.to_vec())),
            (2, Value::Unsigned(self.known_tunnel_server_domains)),
            (
                5,
                Value::TextString(self.operation_hint.as_str().to_owned()),
            ),
        ];
        if let Some(value) = self.current_time {
            fields.push((3, Value::Unsigned(value)));
        }
        if let Some(value) = self.can_use_state_assisted_transactions {
            fields.push((
                4,
                Value::Simple(if value {
                    CborSimpleValue::TrueValue
                } else {
                    CborSimpleValue::FalseValue
                }),
            ));
        }
        encode_map(fields)
    }
}

fn fixed_bytes<const N: usize>(
    value: Value,
    field: &'static str,
) -> Result<[u8; N], ProtocolError> {
    let value = bytes(value, field)?;
    value
        .try_into()
        .map_err(|value: Vec<u8>| ProtocolError::InvalidLength {
            field,
            expected: N,
            actual: value.len(),
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    keyringd_testkit::scenario! {
        group "HybridQrPayload::fmt" {
            case "Concealing QR Secret in Debug output" {
                why "Debug formatting must never disclose the QR secret because logs routinely cross the lifetime and trust boundary of a Hybrid transaction."
                given "Prepare a HybridQrPayLoad" {
                    let value = HybridQrPayload::new([2; 33], [3; 16], 2, HybridOperationHint::GetAssertion);
                }
                when "Shaping QR Payload to a DEBUG string" {
                    let debug = format!("{value:?}");
                }
                then "Concealing QR Secret in Debug output" {
                    assert!(debug.contains("[REDACTED]"));
                    assert!(!debug.contains("3, 3, 3, 3"));
                }
            }
        }
    }
}
