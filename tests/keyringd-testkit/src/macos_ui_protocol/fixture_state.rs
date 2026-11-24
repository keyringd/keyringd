use serde::{Deserialize, Serialize};

use serde_json::Value;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FixtureState {
    pub agent_socket: String,
    pub test_passkey_service: bool,
    pub requests: u64,
    pub pending: u64,
    pub cancelled: u64,
    // Product-owned serialized models; the RPC containers are fully typed.
    pub profiles: Vec<Value>,
    pub fido_credentials: Vec<Value>,
}
