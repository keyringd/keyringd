use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum Evidence {
    Text { name: String, content: String },
    Log { name: String, content: String },
    Bytes { name: String, hexadecimal: String },
    File { name: String, path: String },
    Image { name: String, path: String },
}
