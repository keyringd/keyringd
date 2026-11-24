use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Element {
    pub role: Option<String>,
    pub identifier: Option<String>,
    pub title: Option<String>,
    pub value: Option<String>,
    pub enabled: Option<bool>,
    pub hidden: Option<bool>,
    pub class: String,
}
