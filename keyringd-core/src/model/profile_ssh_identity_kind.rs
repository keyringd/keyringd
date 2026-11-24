use super::{
    Profile,
    ssh_trust::{SshCertificateAuthorityTrustEntry, SshDeviceKeyTrustEntry, SshPasskeyTrustEntry},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProfileSshIdentityKind {
    DeviceCertificate,
    DeviceKey,
    Passkey,
}

impl ProfileSshIdentityKind {
    pub const ALL: [Self; 3] = [Self::DeviceCertificate, Self::DeviceKey, Self::Passkey];

    pub fn display_name(self) -> &'static str {
        match self {
            Self::DeviceCertificate => "Device Certificate",
            Self::DeviceKey => "Device Key",
            Self::Passkey => "Passkey",
        }
    }

    pub fn trust_entry(self, profile: &Profile) -> String {
        match self {
            Self::DeviceCertificate => {
                SshCertificateAuthorityTrustEntry::from_profile(profile).to_string()
            }
            Self::DeviceKey => SshDeviceKeyTrustEntry::from_profile(profile).to_string(),
            Self::Passkey => SshPasskeyTrustEntry::from_profile(profile).to_string(),
        }
    }
}
