use serde::{Deserialize, Serialize};

use super::*;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ControlResponse {
    Ui { result: UiResponse },
    State { state: FixtureState },
    Error { message: String },
}
