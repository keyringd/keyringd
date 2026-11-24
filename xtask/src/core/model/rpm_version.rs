use semver::Version;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RpmVersion {
    version: String,
    release: String,
}

impl From<&Version> for RpmVersion {
    fn from(version: &Version) -> Self {
        let release = if version.pre.is_empty() {
            "1".to_owned()
        } else {
            format!("0.{}.1", version.pre.as_str().replace('-', "."))
        };
        Self {
            version: format!("{}.{}.{}", version.major, version.minor, version.patch),
            release,
        }
    }
}

impl RpmVersion {
    pub(crate) fn version(&self) -> &str {
        &self.version
    }

    pub(crate) fn release(&self) -> &str {
        &self.release
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    keyringd_testkit::scenario! {
        group "RpmVersion::from" {
            case "Splits prerelease" {
                verify "split the prerelease suffix from the RPM version and retain its order" {
                    let version = RpmVersion::from(&Version::parse("0.1.0-alpha.1").unwrap());
                    assert_eq!(version.version(), "0.1.0");
                    assert_eq!(version.release(), "0.alpha.1.1");
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "RpmVersion::from" {
            case "Uses initial release for stable versions" {
                verify "use the initial RPM package release for stable versions" {
                    let version = RpmVersion::from(&Version::parse("1.2.3").unwrap());
                    assert_eq!(version.version(), "1.2.3");
                    assert_eq!(version.release(), "1");
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "RpmVersion::from" {
            case "Normalizes prerelease hyphens and excludes build metadata" {
                verify "normalize prerelease separators and omit build metadata in the RPM version" {
                    let version = RpmVersion::from(&Version::parse("1.2.3-alpha-beta.1+build.7").unwrap());
                    assert_eq!(version.version(), "1.2.3");
                    assert_eq!(version.release(), "0.alpha.beta.1.1");
                }
            }
        }
    }
}
