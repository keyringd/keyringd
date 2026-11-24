/// Transfer only software test credentials between isolated fixture processes.
#[derive(serde::Serialize, serde::Deserialize)]
pub struct TestCredentialSnapshot {
    pub(super) id: Vec<u8>,
    pub(super) signing_key: Vec<u8>,
    pub(super) relying_party_id: String,
    pub(super) user_handle: Vec<u8>,
    pub(super) user_name: Option<String>,
    pub(super) user_display_name: Option<String>,
}
