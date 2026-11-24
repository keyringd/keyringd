use crate::text;
use crate::text::PROFILE_SETUP_REGISTRATION_REQUIRED as REGISTRATION_REQUIRED;
use keyringd_core::application::ProfileSetupProgress;

pub fn progress_message(progress: ProfileSetupProgress) -> &'static str {
    match progress {
        ProfileSetupProgress::Starting => text::PROFILE_SETUP_STARTING,
        ProfileSetupProgress::CreatingPasskey => text::PROFILE_SETUP_CREATING_PASSKEY,
        ProfileSetupProgress::DiscoveringPasskey => text::PROFILE_SETUP_DISCOVERING_PASSKEY,
        ProfileSetupProgress::RecoveringPasskeyPublicKey => {
            text::PROFILE_SETUP_RECOVERING_PASSKEY_PUBLIC_KEY
        }
        ProfileSetupProgress::ProvisioningDeviceKey => text::PROFILE_SETUP_PROVISIONING_DEVICE_KEY,
        ProfileSetupProgress::CommittingProfile => text::PROFILE_SETUP_COMMITTING_PROFILE,
    }
}

pub fn completed_description(name: &str) -> String {
    format!(
        "“{name}” is ready. The keyringd agent can now use its Device Key. {REGISTRATION_REQUIRED}"
    )
}
