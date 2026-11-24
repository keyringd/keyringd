use crate::model::{Profile, ProfileSshIdentity};
use crate::util::base64::encode;
use keyringd_ssh_agent_protocol::SshSignatureAlgorithm;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SshDeviceKeyTrustEntry(String);

impl SshDeviceKeyTrustEntry {
    pub fn from_profile(profile: &Profile) -> Self {
        let blob = profile.device_key().direct_identity();
        Self(format!(
            "{}{} {} {}",
            if profile.device_key().signature_algorithm()
                == SshSignatureAlgorithm::EcdsaSha2NistP256
            {
                ""
            } else if profile.device_key().local_authentication()
                == crate::model::device_key::LocalAuthentication::Required
            {
                "verify-required "
            } else {
                "no-touch-required "
            },
            blob.algorithm(),
            encode(&blob.to_bytes()),
            ProfileSshIdentity::device_key_comment(profile)
        ))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for SshDeviceKeyTrustEntry {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl AsRef<str> for SshDeviceKeyTrustEntry {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::ModelProfileFixture;

    keyringd_testkit::scenario! {
        group "SshDeviceKeyTrustEntry::from_profile" {
            case "Encodes the Device Key with the authentication option required by its signature format" {
                verify "encode verify-required only for the applicable Device Key format and policy" {
                    use crate::model::{ device_key::LocalAuthentication};
                    for (format, authentication, prefix, algorithm) in [
                        (
                            SshSignatureAlgorithm::EcdsaSha2NistP256,
                            LocalAuthentication::NotRequired,
                            "",
                            "ecdsa-sha2-nistp256",
                        ),
                        (
                            SshSignatureAlgorithm::EcdsaSha2NistP256,
                            LocalAuthentication::Required,
                            "",
                            "ecdsa-sha2-nistp256",
                        ),
                        (
                            SshSignatureAlgorithm::SkEcdsaSha2NistP256,
                            LocalAuthentication::NotRequired,
                            "no-touch-required ",
                            "sk-ecdsa-sha2-nistp256@openssh.com",
                        ),
                        (
                            SshSignatureAlgorithm::SkEcdsaSha2NistP256,
                            LocalAuthentication::Required,
                            "verify-required ",
                            "sk-ecdsa-sha2-nistp256@openssh.com",
                        ),
                        (
                            SshSignatureAlgorithm::WebAuthnSkEcdsaSha2NistP256,
                            LocalAuthentication::NotRequired,
                            "no-touch-required ",
                            "sk-ecdsa-sha2-nistp256@openssh.com",
                        ),
                        (
                            SshSignatureAlgorithm::WebAuthnSkEcdsaSha2NistP256,
                            LocalAuthentication::Required,
                            "verify-required ",
                            "sk-ecdsa-sha2-nistp256@openssh.com",
                        ),
                    ] {
                        let profile = ModelProfileFixture {
                            format: format.clone(),
                            authentication,
                            ..Default::default()
                        }
                        .build();
                        let blob = profile.device_key().certificate().public_key();
                        let expected = format!(
                            "{prefix}{algorithm} {} keyringd Work device key@{}",
                            encode(&blob.to_bytes()),
                            gethostname::gethostname().to_string_lossy()
                        );
                        let entry = SshDeviceKeyTrustEntry::from_profile(&profile);
                        assert_eq!(entry.as_str(), expected);
                        assert_eq!(entry.to_string(), expected);
                        assert_eq!(AsRef::<str>::as_ref(&entry), expected);
                    }
                }
            }
        }
    }
}
