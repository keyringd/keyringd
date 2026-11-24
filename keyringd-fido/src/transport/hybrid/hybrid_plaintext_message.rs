use super::{HybridUpdateMessage, TransportError};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HybridPlaintextMessage {
    Shutdown,
    Ctap(Vec<u8>),
    Update(Vec<u8>),
}

impl HybridPlaintextMessage {
    pub fn from_bytes(encoded: &[u8]) -> Result<Self, TransportError> {
        let (&kind, payload) = encoded
            .split_first()
            .ok_or(TransportError::InvalidHybridFrame)?;
        match kind {
            0 if payload.is_empty() => Ok(Self::Shutdown),
            0 | 1 if payload.is_empty() => Err(TransportError::InvalidHybridFrame),
            1 => Ok(Self::Ctap(payload.to_vec())),
            2 => {
                HybridUpdateMessage::from_cbor_bytes(payload)
                    .map_err(|_| TransportError::InvalidHybridFrame)?;
                Ok(Self::Update(payload.to_vec()))
            }
            other => Err(TransportError::UnsupportedHybridMessageType(other)),
        }
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>, TransportError> {
        match self {
            Self::Shutdown => Ok(vec![0]),
            Self::Ctap(payload) if !payload.is_empty() => {
                let mut encoded = Vec::with_capacity(payload.len() + 1);
                encoded.push(1);
                encoded.extend_from_slice(payload);
                Ok(encoded)
            }
            Self::Update(payload) => {
                HybridUpdateMessage::from_cbor_bytes(payload)
                    .map_err(|_| TransportError::InvalidHybridFrame)?;
                let mut encoded = Vec::with_capacity(payload.len() + 1);
                encoded.push(2);
                encoded.extend_from_slice(payload);
                Ok(encoded)
            }
            Self::Ctap(_) => Err(TransportError::InvalidHybridFrame),
        }
    }
}
