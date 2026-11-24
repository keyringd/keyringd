use serde::Deserialize;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AuthenticatorInfo {
    pub id: String,
    pub bus_id: String,
    pub connected: bool,
    pub consent_requests: u64,
}
