use std::fmt;

use semver::Version;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DebVersion(String);

impl From<&Version> for DebVersion {
    fn from(version: &Version) -> Self {
        Self(format!("{}-1", version.to_string().replace('-', "~")))
    }
}

impl DebVersion {
    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for DebVersion {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    keyringd_testkit::scenario! {
        group "DebVersion::from" {
            case "Preserves prerelease order and includes initial package release" {
                verify "render ordered Debian package versions with the initial package revision" {
                    for (input, expected) in [("0.1.0-alpha.1", "0.1.0~alpha.1-1"), ("1.0.0", "1.0.0-1")] {
                        let version = DebVersion::from(&Version::parse(input).unwrap());
                        assert_eq!(version.as_str(), expected);
                        assert_eq!(version.to_string(), expected);
                    }
                }
            }
        }
    }
}
