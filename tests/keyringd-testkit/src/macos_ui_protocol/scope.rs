use serde::{Deserialize, Serialize};

use super::*;

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Scope {
    #[default]
    Application,
    /// Only the window containing this identifier; excludes attached sheets.
    Window {
        anchor: String,
    },
    /// The immediate attached sheet of the window containing this identifier.
    Sheet {
        anchor: String,
    },
    WindowContaining {
        selector: Selector,
    },
    KeyWindow,
}
