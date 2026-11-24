use serde::Deserialize;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum ConsentKind {
    MakeCredential,
    GetAssertion,
    PinUvAuthToken,
    Selection,
}
