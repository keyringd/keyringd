use super::SecurityAutomationAction;
use serde::{Deserialize, Serialize};
#[derive(Deserialize, Serialize)]
pub(super) struct SecurityAutomationRequest {
    pub token: String,
    pub profile_name: String,
    pub action: SecurityAutomationAction,
}
