use super::ConsentKind;
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PendingConsent {
    pub id: u64,
    pub kind: ConsentKind,
    pub rp_id: Option<String>,
    pub user_presence_required: bool,
    pub user_verification_required: bool,
    pub permissions: Vec<String>,
}
impl PendingConsent {
    pub fn grants(&self, permission: &str) -> bool {
        self.permissions
            .iter()
            .any(|candidate| candidate == permission)
    }
}
