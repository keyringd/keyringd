use serde::{Deserialize, Serialize};

use serde_json::Value;

use super::*;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ControlCommand {
    pub ui: Option<UiCommand>,
    pub fail_saves: Option<bool>,
    pub fido_mode: Option<Value>,
    pub fido_credentials: Option<Vec<Value>>,
}

impl ControlCommand {
    pub fn validate(&self) -> Result<(), &'static str> {
        let count = usize::from(self.ui.is_some())
            + usize::from(self.fail_saves.is_some())
            + usize::from(self.fido_mode.is_some())
            + usize::from(self.fido_credentials.is_some());
        if count > 1 {
            Err("only one control operation is allowed")
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    keyringd_testkit::scenario! {
        group "ControlCommand::validate" {
            case "Rejects ambiguous control commands" {
                verify "reject a control payload containing conflicting commands" {
                    std::thread::spawn(|| {
                        assert!(
                            ControlCommand {
                                fail_saves: Some(true),
                                fido_mode: Some(serde_json::json!("hold")),
                                ..Default::default()
                            }
                            .validate()
                            .is_err()
                        );
                    })
                    .join()
                    .unwrap();
                }
            }
        }
    }
}
