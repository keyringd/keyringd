use super::{AuthenticatorAttachment, ResidentKeyRequirement, UserVerificationRequirement};

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthenticatorSelectionCriteria {
    #[serde(skip_serializing_if = "Option::is_none")]
    authenticator_attachment: Option<AuthenticatorAttachment>,
    #[serde(skip_serializing_if = "Option::is_none")]
    resident_key: Option<ResidentKeyRequirement>,
    #[serde(default)]
    require_resident_key: bool,
    #[serde(default)]
    user_verification: UserVerificationRequirement,
}

impl AuthenticatorSelectionCriteria {
    pub fn with_authenticator_attachment(mut self, value: AuthenticatorAttachment) -> Self {
        self.authenticator_attachment = Some(value);
        self
    }
    pub fn with_resident_key(mut self, value: ResidentKeyRequirement) -> Self {
        self.resident_key = Some(value);
        self
    }
    pub fn with_require_resident_key(mut self, value: bool) -> Self {
        self.require_resident_key = value;
        self
    }
    pub fn with_user_verification(mut self, value: UserVerificationRequirement) -> Self {
        self.user_verification = value;
        self
    }
    pub fn authenticator_attachment(&self) -> Option<&AuthenticatorAttachment> {
        self.authenticator_attachment.as_ref()
    }
    pub fn resident_key(&self) -> Option<&ResidentKeyRequirement> {
        self.resident_key.as_ref()
    }
    pub const fn require_resident_key(&self) -> bool {
        self.require_resident_key
    }
    pub fn user_verification(&self) -> &UserVerificationRequirement {
        &self.user_verification
    }
}

impl Default for AuthenticatorSelectionCriteria {
    fn default() -> Self {
        Self {
            authenticator_attachment: None,
            resident_key: None,
            require_resident_key: false,
            user_verification: UserVerificationRequirement::Preferred,
        }
    }
}
