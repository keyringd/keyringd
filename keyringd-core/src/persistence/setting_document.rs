use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SettingDocument {
    pub schema_version: u32,
    pub values: BTreeMap<String, serde_json::Value>,
}

impl Default for SettingDocument {
    fn default() -> Self {
        Self {
            schema_version: 1,
            values: BTreeMap::new(),
        }
    }
}

impl SettingDocument {
    pub fn from_json(value: &[u8]) -> std::io::Result<Self> {
        let document: Self = serde_json::from_slice(value).map_err(invalid_data)?;
        document.validate()?;
        Ok(document)
    }

    pub fn to_json(&self) -> std::io::Result<Vec<u8>> {
        self.validate()?;
        serde_json::to_vec_pretty(self).map_err(invalid_data)
    }

    fn validate(&self) -> std::io::Result<()> {
        if self.schema_version != 1 {
            return Err(invalid_data(format!(
                "unsupported setting schema version: {}",
                self.schema_version
            )));
        }
        Ok(())
    }
}

fn invalid_data(error: impl Into<Box<dyn std::error::Error + Send + Sync>>) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::InvalidData, error)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_corrupt_or_unsupported_documents() {
        for value in [
            b"invalid".as_slice(),
            br#"{"schemaVersion":2,"values":{}}"#,
            br#"{"schemaVersion":1,"values":{},"unexpected":true}"#,
        ] {
            assert_eq!(
                SettingDocument::from_json(value).unwrap_err().kind(),
                std::io::ErrorKind::InvalidData
            );
        }
    }
}
