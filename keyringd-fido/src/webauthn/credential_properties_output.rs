#[derive(Clone, Debug, Default, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct CredentialPropertiesOutput {
    #[serde(rename = "rk", skip_serializing_if = "Option::is_none")]
    rk: Option<bool>,
}

impl CredentialPropertiesOutput {
    pub const fn new(rk: Option<bool>) -> Self {
        Self { rk }
    }

    pub const fn rk(&self) -> Option<bool> {
        self.rk
    }
}
