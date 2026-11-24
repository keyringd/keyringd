use serde::{Deserialize, Serialize};

use super::*;
use std::path::PathBuf;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
pub enum UiCommand {
    Tree,
    Elements {
        scope: Scope,
    },
    FindElement {
        target: Target,
    },
    Press {
        target: Target,
    },
    SetText {
        target: Target,
        text: String,
    },
    ReturnKey {
        target: Target,
    },
    Menu {
        target: Target,
        identifier: String,
    },
    MenuItem {
        target: Target,
        identifier: String,
    },
    ToolbarItem {
        anchor: String,
        identifier: String,
    },
    Identifiers {
        scope: Scope,
        prefix: String,
    },
    DragRow {
        target: Target,
        from: usize,
        to: usize,
    },
    SelectRow {
        target: Target,
        row: usize,
    },
    CloseWindow {
        target: Target,
    },
    Pasteboard,
    ClearPasteboard,
    StatusMenu {
        identifier: String,
    },
    AppMenu {
        submenu: String,
        identifier: String,
    },
    Screenshot {
        path: PathBuf,
        scope: Scope,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    keyringd_testkit::scenario! {
        group "UiCommand::deserialize" {
            case "Rejects unknown command fields" {
                verify "fail control request deserialization when it contains an unknown field" {
                    std::thread::spawn(|| {
                    assert!(serde_json::from_str::<UiCommand>(r#"{"op":"press","target":{"scope":{"kind":"application"},"selector":{"kind":"id","value":"ok"}},"typo":true}"#).is_err());
                    }).join().unwrap();
                }
            }
        }
    }
}
