use p256::ecdsa::SigningKey;

pub(super) struct TestCredential {
    pub signing_key: SigningKey,
    pub relying_party_id: String,
    pub user_handle: Vec<u8>,
    pub user_name: Option<String>,
    pub user_display_name: Option<String>,
}
