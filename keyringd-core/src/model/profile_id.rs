use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub struct ProfileId(String);

impl ProfileId {
    pub fn new() -> Self {
        let mut random = [0; 16];
        getrandom::fill(&mut random).expect("the operating system random source must be available");
        Self(random.iter().map(|byte| format!("{byte:02x}")).collect())
    }

    pub fn from_stored(value: String) -> Self {
        Self(value)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn is_valid(&self) -> bool {
        !self.0.is_empty()
            && self
                .0
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
    }
}

impl Default for ProfileId {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    keyringd_testkit::scenario! {
        group "ProfileId::is_valid" {
            case "Validates stored IDs without normalizing their contents" {
                verify "retain the original bytes and reject invalid persisted ID spellings" {
                    for value in ["a", "A-Z-019", "-"] {
                        let id = ProfileId::from_stored(value.into());
                        assert!(id.is_valid(), "{value:?}");
                        assert_eq!(id.as_str(), value);
                    }
                    for value in [
                        "",
                        "with space",
                        " leading",
                        "trailing ",
                        "a_b",
                        "a/b",
                        "café",
                        "a\n",
                    ] {
                        let id = ProfileId::from_stored(value.into());
                        assert!(!id.is_valid(), "{value:?}");
                        assert_eq!(id.as_str(), value);
                    }
                    let generated = ProfileId::new();
                    assert!(generated.is_valid());
                    assert_eq!(generated.as_str().len(), 32);
                    assert!(
                        generated
                            .as_str()
                            .bytes()
                            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
                    );
                }
            }
        }
    }
}
