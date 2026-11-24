use crate::text;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProfileSetupStep {
    Create,
    Find,
    RecoverPublicKey,
    AuthorizeNew,
    AuthorizeExisting,
}

impl ProfileSetupStep {
    pub fn for_progress(
        progress: keyringd_core::application::ProfileSetupProgress,
        choice: keyringd_core::application::PasskeyChoice,
    ) -> Self {
        use keyringd_core::application::{PasskeyChoice, ProfileSetupProgress};
        match progress {
            ProfileSetupProgress::Starting | ProfileSetupProgress::CreatingPasskey => {
                match choice {
                    PasskeyChoice::CreateNew => Self::Create,
                    PasskeyChoice::Existing => Self::Find,
                }
            }
            ProfileSetupProgress::DiscoveringPasskey => Self::Find,
            ProfileSetupProgress::RecoveringPasskeyPublicKey => Self::RecoverPublicKey,
            ProfileSetupProgress::ProvisioningDeviceKey
            | ProfileSetupProgress::CommittingProfile => match choice {
                PasskeyChoice::CreateNew => Self::AuthorizeNew,
                PasskeyChoice::Existing => Self::AuthorizeExisting,
            },
        }
    }

    pub fn purpose(self) -> crate::key_operation::AuthenticatorPromptPurpose {
        use crate::key_operation::AuthenticatorPromptPurpose;
        match self {
            Self::Create => AuthenticatorPromptPurpose::CreatePasskey,
            Self::Find => AuthenticatorPromptPurpose::FindExistingPasskey,
            Self::RecoverPublicKey => AuthenticatorPromptPurpose::RecoverPasskeyPublicKey,
            Self::AuthorizeNew => AuthenticatorPromptPurpose::AuthorizeWithNewPasskey,
            Self::AuthorizeExisting => AuthenticatorPromptPurpose::AuthorizeWithExistingPasskey,
        }
    }

    pub fn for_purpose(context: &crate::key_operation::AuthenticatorPromptPurpose) -> Option<Self> {
        use crate::key_operation::AuthenticatorPromptPurpose;
        match context {
            AuthenticatorPromptPurpose::CreatePasskey => Some(Self::Create),
            AuthenticatorPromptPurpose::FindExistingPasskey => Some(Self::Find),
            AuthenticatorPromptPurpose::RecoverPasskeyPublicKey => Some(Self::RecoverPublicKey),
            AuthenticatorPromptPurpose::AuthorizeWithNewPasskey => Some(Self::AuthorizeNew),
            AuthenticatorPromptPurpose::AuthorizeWithExistingPasskey => {
                Some(Self::AuthorizeExisting)
            }
            _ => None,
        }
    }

    pub fn heading(self) -> &'static str {
        match self {
            Self::Create => text::PROFILE_SETUP_CREATE_HEADING,
            Self::Find => text::PROFILE_SETUP_FIND_HEADING,
            Self::RecoverPublicKey => text::PROFILE_SETUP_RECOVER_PUBLIC_KEY_HEADING,
            Self::AuthorizeNew => text::PROFILE_SETUP_AUTHORIZE_NEW_HEADING,
            Self::AuthorizeExisting => text::PROFILE_SETUP_AUTHORIZE_EXISTING_HEADING,
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Self::Create => text::PROFILE_SETUP_CREATE_DESCRIPTION,
            Self::Find => text::PROFILE_SETUP_FIND_DESCRIPTION,
            Self::RecoverPublicKey => text::PROFILE_SETUP_RECOVER_PUBLIC_KEY_DESCRIPTION,
            Self::AuthorizeNew => text::PROFILE_SETUP_AUTHORIZE_NEW_DESCRIPTION,
            Self::AuthorizeExisting => text::PROFILE_SETUP_AUTHORIZE_DESCRIPTION,
        }
    }
    pub fn scan_instruction(self) -> &'static str {
        match self {
            Self::Create => text::PROFILE_SETUP_CREATE_SCAN,
            Self::Find => text::PROFILE_SETUP_FIND_SCAN,
            Self::RecoverPublicKey => text::PROFILE_SETUP_RECOVER_PUBLIC_KEY_SCAN,
            Self::AuthorizeNew => text::PROFILE_SETUP_AUTHORIZE_NEW_SCAN,
            Self::AuthorizeExisting => text::PROFILE_SETUP_AUTHORIZE_EXISTING_SCAN,
        }
    }
}
