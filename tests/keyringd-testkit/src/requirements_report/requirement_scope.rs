use serde::Deserialize;

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub(super) enum RequirementScope {
    #[default]
    In,
    Boundary,
    OtherComponent,
    OutOfScope,
    Deferred,
}

impl RequirementScope {
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::In => "IN",
            Self::Boundary => "BOUNDARY",
            Self::OtherComponent => "OTHER COMPONENT",
            Self::OutOfScope => "OUT OF SCOPE",
            Self::Deferred => "DEFERRED",
        }
    }

    pub(super) fn requires_reason(self) -> bool {
        matches!(
            self,
            Self::OtherComponent | Self::OutOfScope | Self::Deferred
        )
    }

    pub(super) fn expects_tests(self) -> bool {
        matches!(self, Self::In | Self::Boundary)
    }
}
