use crate::transport::hybrid::test_support::HybridQrDecodeError;
use keyringd_fido::ProtocolError;
use keyringd_fido::transport::hybrid::HybridQrPayload;

/// Validates the complete wire representation without exposing its secret fields.
pub(crate) fn validate_qr_uri(uri: &str) -> Result<(), HybridQrDecodeError> {
    let digits = uri
        .strip_prefix("FIDO:/")
        .ok_or(HybridQrDecodeError::InvalidScheme)?;
    let encoded = digit_decode(digits)?;
    validate_qr_payload(&encoded)
}

pub(crate) fn digit_encode(input: &[u8]) -> String {
    const PARTIAL_CHUNK_DIGITS: usize = 0x0fda8530;
    let mut output = String::new();
    for bytes in input.chunks(7) {
        let mut chunk = [0u8; 8];
        chunk[..bytes.len()].copy_from_slice(bytes);
        let value = u64::from_le_bytes(chunk);
        let digits = if bytes.len() == 7 {
            17
        } else {
            0x0f & (PARTIAL_CHUNK_DIGITS >> (4 * bytes.len()))
        };
        output.push_str(&format!("{value:0digits$}"));
    }
    output
}

pub(crate) fn digit_decode(input: &str) -> Result<Vec<u8>, HybridQrDecodeError> {
    const PARTIAL_LENGTHS: &[(usize, usize)] = &[(3, 1), (5, 2), (8, 3), (10, 4), (13, 5), (15, 6)];
    if input.is_empty() || !input.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(if input.is_empty() {
            HybridQrDecodeError::InvalidDigitLength
        } else {
            HybridQrDecodeError::InvalidDigit
        });
    }
    let full_chunks = input.len() / 17;
    let remainder = input.len() % 17;
    let partial_bytes = if remainder == 0 {
        0
    } else {
        PARTIAL_LENGTHS
            .iter()
            .find_map(|(digits, bytes)| (*digits == remainder).then_some(*bytes))
            .ok_or(HybridQrDecodeError::InvalidDigitLength)?
    };
    let mut result = Vec::with_capacity(full_chunks * 7 + partial_bytes);
    for (index, chunk) in input.as_bytes().chunks(17).enumerate() {
        let text = std::str::from_utf8(chunk).map_err(|_| HybridQrDecodeError::InvalidDigit)?;
        let value = text
            .parse::<u64>()
            .map_err(|_| HybridQrDecodeError::NonCanonicalDigitChunk)?;
        let byte_count = if index < full_chunks {
            7
        } else {
            partial_bytes
        };
        let limit = 1u64 << (byte_count * 8);
        if value >= limit {
            return Err(HybridQrDecodeError::NonCanonicalDigitChunk);
        }
        result.extend_from_slice(&value.to_le_bytes()[..byte_count]);
    }
    Ok(result)
}

pub(crate) fn validate_qr_payload(encoded: &[u8]) -> Result<(), HybridQrDecodeError> {
    let payload = HybridQrPayload::from_cbor_bytes(encoded).map_err(map_qr_codec_error)?;
    if !matches!(payload.public_key()[0], 0x02 | 0x03) {
        return Err(HybridQrDecodeError::InvalidPublicKey);
    }
    p256::PublicKey::from_sec1_bytes(payload.public_key())
        .map_err(|_| HybridQrDecodeError::InvalidPublicKey)?;
    Ok(())
}

pub(crate) fn map_qr_codec_error(error: ProtocolError) -> HybridQrDecodeError {
    match error {
        ProtocolError::MissingCtapField(key) => HybridQrDecodeError::MissingField(key),
        ProtocolError::InvalidLength {
            field: "hybrid QR public key",
            ..
        } => HybridQrDecodeError::InvalidPublicKey,
        ProtocolError::InvalidLength {
            field: "hybrid QR secret",
            ..
        } => HybridQrDecodeError::InvalidSecret,
        ProtocolError::UnexpectedType {
            field: "hybrid QR operation hint",
            ..
        } => HybridQrDecodeError::InvalidHint,
        ProtocolError::Codec(keyringd_fido::CodecError::InvalidCbor(_)) => {
            HybridQrDecodeError::InvalidCbor
        }
        _ => HybridQrDecodeError::InvalidField,
    }
}
