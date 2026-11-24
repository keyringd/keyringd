use p256::ecdsa::SigningKey;

use keyringd_fido::CredentialProtectionPolicy;

pub(crate) struct ModelCredential {
    pub signing_key: SigningKey,
    pub relying_party_id: String,
    pub user_handle: Vec<u8>,
    pub discoverable: bool,
    pub protection: CredentialProtectionPolicy,
    pub creation_order: u64,
}
