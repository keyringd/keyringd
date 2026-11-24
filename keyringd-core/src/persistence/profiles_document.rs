use serde::{Deserialize, Serialize};

use super::{ProfileSaveError, ProfileValidationError};
use crate::model::Profile;

pub const PROFILES_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProfilesDocument {
    pub schema_version: u32,
    pub profiles: Vec<Profile>,
}

impl Default for ProfilesDocument {
    fn default() -> Self {
        Self {
            schema_version: PROFILES_SCHEMA_VERSION,
            profiles: Vec::new(),
        }
    }
}

impl ProfilesDocument {
    pub fn from_json(value: &[u8]) -> std::io::Result<Self> {
        let document: Self = serde_json::from_slice(value)
            .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
        document
            .validate()
            .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
        Ok(document)
    }

    pub fn to_json(&self) -> Result<Vec<u8>, ProfileSaveError> {
        self.validate()?;
        Ok(serde_json::to_vec_pretty(self).map_err(std::io::Error::other)?)
    }

    pub fn validate(&self) -> Result<(), ProfileValidationError> {
        if self.schema_version != PROFILES_SCHEMA_VERSION {
            return Err(ProfileValidationError::UnsupportedSchemaVersion(
                self.schema_version,
            ));
        }
        let mut ids = std::collections::HashSet::new();
        for profile in &self.profiles {
            profile
                .validate()
                .map_err(ProfileValidationError::InvalidProfile)?;
            if !ids.insert(profile.id()) {
                return Err(ProfileValidationError::DuplicateId);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    keyringd_testkit::scenario! {
        group "ProfilesDocument::from_json" {
            case "Json conversion rejects malformed data and unsupported schema versions" {
                verify "fail deserialization or validation for invalid documents and unknown schema versions" {
                    for value in [
                        b"not JSON".as_slice(),
                        br#"{"schemaVersion":0,"profiles":[]}"#,
                    ] {
                        let error = ProfilesDocument::from_json(value).err().unwrap();
                        assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
                    }
                    let document = ProfilesDocument {
                        schema_version: 0,
                        ..ProfilesDocument::default()
                    };
                    assert!(matches!(document.to_json(), Err(ProfileSaveError::InvalidDocument(
                        ProfileValidationError::UnsupportedSchemaVersion(0)
                    ))));
                }
            }
        }
    }
}
