use std::time::SystemTime;

use super::{Profile, ProfileSshIdentityKind};
use crate::util::base64::encode;
use keyringd_ssh_agent_protocol::{SshAgentIdentity, SshPublicKeyBlob};

/// One SSH public key or certificate selected from a profile, with its comment.
/// Unlike the protocol's SshAgentIdentity, this also identifies its role in the profile.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProfileSshIdentity {
    kind: ProfileSshIdentityKind,
    key_blob: SshPublicKeyBlob,
    comment: String,
}

impl ProfileSshIdentity {
    pub fn from_profile_at(
        profile: &Profile,
        kind: ProfileSshIdentityKind,
        reference_time: SystemTime,
    ) -> Option<Self> {
        let (key_blob, comment) = match kind {
            ProfileSshIdentityKind::DeviceCertificate => {
                let certificate = profile.device_key().certificate();
                if !certificate.is_within_validity_period(reference_time) {
                    return None;
                }
                (
                    certificate.to_key_blob(),
                    format!("keyringd {} device certificate", profile.display_name()),
                )
            }
            ProfileSshIdentityKind::DeviceKey => (
                profile
                    .device_key()
                    .certificate()
                    .public_key()
                    .into_key_blob(),
                Self::device_key_comment(profile),
            ),
            ProfileSshIdentityKind::Passkey => (
                profile.passkey().to_ssh_public_key().into_key_blob(),
                format!("keyringd {} passkey", profile.display_name()),
            ),
        };
        Some(Self {
            kind,
            key_blob,
            comment,
        })
    }

    pub fn available_kinds_at(
        profile: &Profile,
        reference_time: SystemTime,
    ) -> Vec<ProfileSshIdentityKind> {
        let mut kinds = Vec::with_capacity(3);
        if profile
            .device_key()
            .certificate()
            .is_within_validity_period(reference_time)
        {
            kinds.push(ProfileSshIdentityKind::DeviceCertificate);
        }
        kinds.push(ProfileSshIdentityKind::DeviceKey);
        kinds.push(ProfileSshIdentityKind::Passkey);
        kinds
    }

    pub fn kind(&self) -> ProfileSshIdentityKind {
        self.kind
    }

    pub fn key_blob(&self) -> &SshPublicKeyBlob {
        &self.key_blob
    }

    pub fn to_public_key_line(&self) -> String {
        format!(
            "{} {} {}",
            self.key_blob.algorithm(),
            encode(&self.key_blob.to_bytes()),
            self.comment
        )
    }

    pub fn into_agent_identity(self) -> SshAgentIdentity {
        SshAgentIdentity::new(self.key_blob, self.comment)
    }

    pub(crate) fn device_key_comment(profile: &Profile) -> String {
        format!(
            "keyringd {} device key@{}",
            profile.display_name(),
            gethostname::gethostname().to_string_lossy()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::ModelProfileFixture;

    keyringd_testkit::scenario! {
        group "ProfileSshIdentity::from_profile_at" {
            case "Selects certificate identities only within their usable time interval" {
                verify "include the certificate at valid_after and exclude it at valid_before" {
                    use std::time::{Duration, UNIX_EPOCH};
                    let profile = ModelProfileFixture::default().build();
                    for (seconds, usable) in [(99, false), (100, true), (199, true), (200, false)] {
                        let time = UNIX_EPOCH + Duration::from_secs(seconds);
                        let certificate = ProfileSshIdentity::from_profile_at(
                            &profile,
                            ProfileSshIdentityKind::DeviceCertificate,
                            time,
                        );
                        assert_eq!(certificate.is_some(), usable, "{seconds}");
                        let kinds = ProfileSshIdentity::available_kinds_at(&profile, time);
                        let mut expected = vec![
                            ProfileSshIdentityKind::DeviceKey,
                            ProfileSshIdentityKind::Passkey,
                        ];
                        if usable {
                            expected.insert(0, ProfileSshIdentityKind::DeviceCertificate);
                        }
                        assert_eq!(kinds, expected);
                        assert!(
                            ProfileSshIdentity::from_profile_at(&profile, ProfileSshIdentityKind::DeviceKey, time)
                                .is_some()
                        );
                        assert!(
                            ProfileSshIdentity::from_profile_at(&profile, ProfileSshIdentityKind::Passkey, time)
                                .is_some()
                        );
                    }
                    let before_epoch = UNIX_EPOCH - Duration::from_secs(1);
                    assert!(
                        ProfileSshIdentity::from_profile_at(
                            &profile,
                            ProfileSshIdentityKind::DeviceCertificate,
                            before_epoch
                        )
                        .is_none()
                    );
                }
            }
        }
        group "ProfileSshIdentity::to_public_key_line" {
            case "Keeps the selected key and comment when exporting public identities" {
                verify "preserve the key blob and profile comment in the exported identity" {
                    use std::time::{Duration, UNIX_EPOCH};
                    let profile = ModelProfileFixture::default().build();
                    let hostname = gethostname::gethostname().to_string_lossy().into_owned();
                    for (kind, blob, comment) in [
                        (
                            ProfileSshIdentityKind::DeviceCertificate,
                            profile.device_key().certificate().to_key_blob(),
                            "keyringd Work device certificate".to_owned(),
                        ),
                        (
                            ProfileSshIdentityKind::DeviceKey,
                            profile.device_key().certificate().public_key().into_key_blob(),
                            format!("keyringd Work device key@{hostname}"),
                        ),
                        (
                            ProfileSshIdentityKind::Passkey,
                            profile.passkey().to_ssh_public_key().into_key_blob(),
                            "keyringd Work passkey".to_owned(),
                        ),
                    ] {
                        let identity = ProfileSshIdentity::from_profile_at(
                            &profile,
                            kind,
                            UNIX_EPOCH + Duration::from_secs(150),
                        )
                        .unwrap();
                        assert_eq!(identity.kind(), kind);
                        assert_eq!(identity.key_blob(), &blob);
                        assert_eq!(
                            identity.to_public_key_line(),
                            format!(
                                "{} {} {}",
                                blob.algorithm(),
                                encode(&blob.to_bytes()),
                                comment
                            )
                        );
                        let agent = identity.into_agent_identity();
                        assert_eq!(agent.key_blob(), &blob);
                        assert_eq!(agent.comment(), comment);
                    }
                }
            }
        }
    }
}
