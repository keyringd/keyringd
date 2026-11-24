#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum HybridOperationHint {
    GetAssertion,
    MakeCredential,
}

impl HybridOperationHint {
    pub fn from_wire_value(value: &str) -> Self {
        match value {
            "mc" => Self::MakeCredential,
            // CTAP 2.2 requires implementations to treat unknown values as `ga`.
            "ga" => Self::GetAssertion,
            _ => Self::GetAssertion,
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::GetAssertion => "ga",
            Self::MakeCredential => "mc",
        }
    }
}
