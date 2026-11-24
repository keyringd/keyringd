/// Why an authenticator prompt is shown, including its security-key selection policy.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AuthenticatorPromptPurpose {
    CreatePasskey,
    AuthorizeWithNewPasskey,
    FindExistingPasskey,
    RecoverPasskeyPublicKey,
    AuthorizeWithExistingPasskey,
    DeviceKeyProtectionChange { profile_name: String },
    DeviceKeyReplacement { profile_name: String },
    AgentRequest,
}

impl AuthenticatorPromptPurpose {
    pub fn for_profile_setup(
        progress: keyringd_core::application::ProfileSetupProgress,
        passkey: keyringd_core::application::PasskeyChoice,
    ) -> Option<Self> {
        use keyringd_core::application::{PasskeyChoice, ProfileSetupProgress};
        match (passkey, progress) {
            (PasskeyChoice::CreateNew, ProfileSetupProgress::CreatingPasskey) => {
                Some(Self::CreatePasskey)
            }
            (PasskeyChoice::CreateNew, ProfileSetupProgress::ProvisioningDeviceKey) => {
                Some(Self::AuthorizeWithNewPasskey)
            }
            (PasskeyChoice::Existing, ProfileSetupProgress::DiscoveringPasskey) => {
                Some(Self::FindExistingPasskey)
            }
            (PasskeyChoice::Existing, ProfileSetupProgress::RecoveringPasskeyPublicKey) => {
                Some(Self::RecoverPasskeyPublicKey)
            }
            (PasskeyChoice::Existing, ProfileSetupProgress::ProvisioningDeviceKey) => {
                Some(Self::AuthorizeWithExistingPasskey)
            }
            _ => None,
        }
    }
    pub fn allows_security_key_choice(&self) -> bool {
        matches!(
            self,
            Self::CreatePasskey
                | Self::FindExistingPasskey
                | Self::DeviceKeyProtectionChange { .. }
                | Self::DeviceKeyReplacement { .. }
                | Self::AgentRequest
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    keyringd_testkit::scenario! {
        group "AuthenticatorPromptPurpose::allows_security_key_choice" {
            case "Independent operations allow transport selection but bound setup steps do not" {
                verify "offer a transport chooser only when setup has not bound a transport" {
                    for context in [
                        AuthenticatorPromptPurpose::CreatePasskey,
                        AuthenticatorPromptPurpose::FindExistingPasskey,
                        AuthenticatorPromptPurpose::DeviceKeyProtectionChange {
                            profile_name: "Work".into(),
                        },
                        AuthenticatorPromptPurpose::DeviceKeyReplacement {
                            profile_name: "Work".into(),
                        },
                        AuthenticatorPromptPurpose::AgentRequest,
                    ] {
                        assert!(context.allows_security_key_choice(), "{context:?}");
                    }
                    for context in [
                        AuthenticatorPromptPurpose::AuthorizeWithNewPasskey,
                        AuthenticatorPromptPurpose::RecoverPasskeyPublicKey,
                        AuthenticatorPromptPurpose::AuthorizeWithExistingPasskey,
                    ] {
                        assert!(!context.allows_security_key_choice(), "{context:?}");
                    }
                }
            }
        }
    }
}
