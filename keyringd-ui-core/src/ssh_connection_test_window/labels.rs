use crate::text;
use keyringd_core::model::ProfileSshIdentityKind;

pub fn identity_title(identity: ProfileSshIdentityKind) -> &'static str {
    match identity {
        ProfileSshIdentityKind::DeviceCertificate => text::SSH_CONNECTION_TEST_CERTIFICATE_CHOICE,
        ProfileSshIdentityKind::DeviceKey => text::SSH_CONNECTION_TEST_DEVICE_KEY_CHOICE,
        ProfileSshIdentityKind::Passkey => text::SSH_CONNECTION_TEST_PASSKEY_CHOICE,
    }
}

pub fn identity_description(
    identity: ProfileSshIdentityKind,
    device_key_description: &'static str,
) -> &'static str {
    match identity {
        ProfileSshIdentityKind::DeviceCertificate => {
            text::SSH_CONNECTION_TEST_CERTIFICATE_DESCRIPTION
        }
        ProfileSshIdentityKind::DeviceKey => device_key_description,
        ProfileSshIdentityKind::Passkey => text::SSH_CONNECTION_TEST_PASSKEY_DESCRIPTION,
    }
}

pub fn host_label(target: &str) -> String {
    format!("Host: {target}")
}

pub fn profile_heading(name: &str) -> String {
    format!("Test SSH Connection — {name}")
}
