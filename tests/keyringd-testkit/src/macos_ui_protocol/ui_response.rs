use serde::{Deserialize, Serialize};

use super::*;
use std::path::PathBuf;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum UiResponse {
    Tree { tree: String },
    // Vec makes the field required: [] alone means absence, >1 is ambiguous.
    Elements { elements: Vec<Element> },
    Pressed { accepted: bool },
    Text { value: String },
    Done,
    Pasteboard { text: Option<String> },
    Identifiers { identifiers: Vec<String> },
    Screenshot { path: PathBuf },
}

#[cfg(test)]
mod tests {
    use super::*;
    keyringd_testkit::scenario! {
        group "UiResponse::deserialize" {
            case "Validates element response shape" {
                verify "decode valid element responses and reject malformed shapes" {
                    std::thread::spawn(|| {
                        assert!(serde_json::from_str::<UiResponse>(r#"{"kind":"elements"}"#).is_err());
                        assert!(
                            serde_json::from_str::<UiResponse>(r#"{"kind":"elements","elements":[{}]}"#).is_err()
                        );
                        assert!(serde_json::from_str::<UiResponse>(r#"{"kind":"elements","elements":[]}"#).is_ok());
                    })
                    .join()
                    .unwrap();
                }
            }
        }
    }
}
