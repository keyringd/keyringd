/// An application-generated challenge, before conversion to ClientData.
///
/// Each generation produces 32 fresh random bytes.
/// Keep the generated value for the lifetime of one request, including transport
/// retries. The ClientData provider wraps these bytes in clientDataJSON for the
/// WebAuthn format, or uses them directly for the SSH format.
pub(crate) struct Challenge([u8; 32]);

impl Challenge {
    pub(crate) fn generate() -> Self {
        let mut bytes = [0; 32];
        getrandom::fill(&mut bytes).expect("the operating system random source must be available");
        Self(bytes)
    }

    pub(crate) fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    pub(crate) fn into_bytes(self) -> Vec<u8> {
        self.0.to_vec()
    }
}
