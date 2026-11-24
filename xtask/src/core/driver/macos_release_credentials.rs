use anyhow::{Context, Result, ensure};

use crate::core::driver::MacosTools;

// Deliberately no Debug implementation: this contains release secrets.
pub(crate) struct MacosReleaseCredentials {
    pub certificate: String,
    pub certificate_password: String,
    pub profile: String,
    pub apple_id: String,
    pub team: String,
    pub apple_password: String,
}

impl MacosReleaseCredentials {
    pub(crate) fn from_env(tools: &mut MacosTools) -> Result<Self> {
        let credentials = Self::read(|name| {
            std::env::var(name).with_context(|| format!("required release setting missing: {name}"))
        })?;
        tools.mask(&credentials.certificate_password);
        tools.mask(&credentials.apple_password);
        Ok(credentials)
    }

    fn read(mut read: impl FnMut(&str) -> Result<String>) -> Result<Self> {
        let mut required = |name| {
            let value = read(name)?;
            ensure!(
                !value.trim().is_empty(),
                "required release setting missing: {name}"
            );
            Ok(value)
        };
        Ok(Self {
            certificate: required("MACOS_CERTIFICATE_BASE64")?,
            certificate_password: required("MACOS_CERTIFICATE_PASSWORD")?,
            profile: required("MACOS_PROVISION_PROFILE_BASE64")?,
            apple_id: required("APPLE_ID")?,
            team: required("APPLE_TEAM_ID")?,
            apple_password: required("APPLE_APP_SPECIFIC_PASSWORD")?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    keyringd_testkit::scenario! {
        group "MacosReleaseCredentials::read" {
            case "Every release setting is required" {
                verify "fail to read release credentials when any required setting is missing" {
                    for missing in [
                        "MACOS_CERTIFICATE_BASE64",
                        "MACOS_CERTIFICATE_PASSWORD",
                        "MACOS_PROVISION_PROFILE_BASE64",
                        "APPLE_ID",
                        "APPLE_TEAM_ID",
                        "APPLE_APP_SPECIFIC_PASSWORD",
                    ] {
                        assert!(
                            MacosReleaseCredentials::read(|name| Ok(
                                if name == missing { " " } else { "present" }.into()
                            ))
                            .is_err()
                        );
                    }
                }
            }
        }
    }
}
