use super::ProtocolError;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct AuthenticatorGetInfoRequest;

impl AuthenticatorGetInfoRequest {
    pub const fn new() -> Self {
        Self
    }

    pub fn from_bytes(payload: &[u8]) -> Result<Self, ProtocolError> {
        if payload.is_empty() {
            Ok(Self)
        } else {
            Err(ProtocolError::UnexpectedType {
                field: "authenticatorGetInfo request",
                expected: "an empty payload",
            })
        }
    }

    pub const fn to_bytes(self) -> &'static [u8] {
        &[]
    }
}
