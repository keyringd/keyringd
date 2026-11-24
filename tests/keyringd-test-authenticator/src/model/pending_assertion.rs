use p256::ecdsa::SigningKey;

pub(super) struct PendingAssertion {
    pub(super) credential_id: Vec<u8>,
    pub(super) user_handle: Vec<u8>,
    pub(super) signing_key: SigningKey,
    pub(super) discoverable: bool,
    pub(super) relying_party_id: String,
    pub(super) client_data_hash: [u8; 32],
    pub(super) user_verified: bool,
}
