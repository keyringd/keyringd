/// Whether use of a Device Key requires local authentication on this computer.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub enum LocalAuthentication {
    #[default]
    NotRequired,
    Required,
}
