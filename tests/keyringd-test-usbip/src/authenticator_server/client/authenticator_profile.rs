use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum AuthenticatorProfile {
    UvConfiguredPinConfigured,
    #[default]
    UvConfiguredPinUnconfigured,
    UvUnconfiguredPinConfigured,
    UvUnconfiguredPinUnconfigured,
}
impl AuthenticatorProfile {
    pub(crate) fn path(self) -> &'static str {
        match self {
            Self::UvConfiguredPinConfigured => "uv-configured/pin-configured",
            Self::UvConfiguredPinUnconfigured => "uv-configured/pin-unconfigured",
            Self::UvUnconfiguredPinConfigured => "uv-unconfigured/pin-configured",
            Self::UvUnconfiguredPinUnconfigured => "uv-unconfigured/pin-unconfigured",
        }
    }
}
