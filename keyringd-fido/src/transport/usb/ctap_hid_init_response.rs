use super::{CtapHidCapabilities, TransportError};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CtapHidInitResponse {
    nonce: [u8; 8],
    channel_id: u32,
    protocol_version: u8,
    device_version_major: u8,
    device_version_minor: u8,
    device_version_build: u8,
    capabilities: CtapHidCapabilities,
}

impl CtapHidInitResponse {
    pub fn parse(payload: &[u8]) -> Result<Self, TransportError> {
        if payload.len() < 17 {
            return Err(TransportError::InvalidInitResponseLength {
                actual: payload.len(),
            });
        }
        Ok(Self {
            nonce: payload[..8].try_into().expect("fixed range"),
            channel_id: u32::from_be_bytes(payload[8..12].try_into().expect("fixed range")),
            protocol_version: payload[12],
            device_version_major: payload[13],
            device_version_minor: payload[14],
            device_version_build: payload[15],
            capabilities: CtapHidCapabilities::from_bits_retain(payload[16]),
        })
    }
    pub const fn nonce(&self) -> &[u8; 8] {
        &self.nonce
    }
    pub const fn channel_id(&self) -> u32 {
        self.channel_id
    }
    pub const fn protocol_version(&self) -> u8 {
        self.protocol_version
    }
    pub const fn device_version_major(&self) -> u8 {
        self.device_version_major
    }
    pub const fn device_version_minor(&self) -> u8 {
        self.device_version_minor
    }
    pub const fn device_version_build(&self) -> u8 {
        self.device_version_build
    }
    pub const fn capabilities(&self) -> CtapHidCapabilities {
        self.capabilities
    }
}
