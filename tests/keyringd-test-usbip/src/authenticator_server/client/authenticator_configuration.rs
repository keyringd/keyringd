use super::{AuthenticatorProfile, LargeBlobSupport};
use serde::Serialize;

#[derive(Clone, Copy, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthenticatorConfiguration {
    pub profile: AuthenticatorProfile,
    pub manual_approval: bool,
    pub large_blob_support: LargeBlobSupport,
}
