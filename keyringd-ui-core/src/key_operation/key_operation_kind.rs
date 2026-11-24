use super::AuthenticatorPromptPurpose;
use keyringd_core::application::PasskeyChoice;

#[derive(Clone, Debug)]
/// User operation represented by one key-operation flow.
pub enum KeyOperationKind {
    ProfileSetup { passkey: PasskeyChoice },
    DeviceKeyProtectionChange { profile_name: String },
    DeviceKeyReplacement { profile_name: String },
    AgentRequest,
}

impl KeyOperationKind {
    /// Profile setup derives its purpose from progress in the owning window.
    pub fn authenticator_prompt_purpose(&self) -> Option<AuthenticatorPromptPurpose> {
        Some(match self {
            Self::ProfileSetup { .. } => return None,
            Self::DeviceKeyProtectionChange { profile_name } => {
                AuthenticatorPromptPurpose::DeviceKeyProtectionChange {
                    profile_name: profile_name.clone(),
                }
            }
            Self::DeviceKeyReplacement { profile_name } => {
                AuthenticatorPromptPurpose::DeviceKeyReplacement {
                    profile_name: profile_name.clone(),
                }
            }
            Self::AgentRequest => AuthenticatorPromptPurpose::AgentRequest,
        })
    }
}
