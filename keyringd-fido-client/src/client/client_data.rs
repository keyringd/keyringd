use keyringd_fido::CollectedClientData;

/// Exact client-data bytes, normally JSON for WebAuthn or protocol-specific bytes
/// for other clients. Results retain the bytes used for the ceremony.
#[derive(Clone, Eq, PartialEq)]
pub struct ClientData(Vec<u8>);

impl ClientData {
    pub fn from_bytes(bytes: Vec<u8>) -> Self {
        Self(bytes)
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

impl TryFrom<CollectedClientData> for ClientData {
    type Error = keyringd_fido::ProtocolError;

    fn try_from(value: CollectedClientData) -> Result<Self, Self::Error> {
        value.to_json_bytes().map(Self)
    }
}

impl std::fmt::Debug for ClientData {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ClientData")
            .field("length", &self.0.len())
            .finish_non_exhaustive()
    }
}
