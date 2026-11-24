use crate::model::Profile;
use crate::util::base64::encode;
#[cfg(test)]
use keyringd_ssh_agent_protocol::SshSignatureAlgorithm;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SshPasskeyTrustEntry(String);

impl SshPasskeyTrustEntry {
    pub fn from_profile(profile: &Profile) -> Self {
        let blob = profile.passkey().to_ssh_public_key();
        Self(format!(
            "sk-ecdsa-sha2-nistp256@openssh.com {} keyringd {} passkey",
            encode(&blob.to_bytes()),
            profile.display_name()
        ))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for SshPasskeyTrustEntry {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl AsRef<str> for SshPasskeyTrustEntry {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::ModelProfileFixture;

    keyringd_testkit::scenario! {
        group "SshPasskeyTrustEntry::from_profile" {
            case "Registers the Passkey independently of Device Key authentication settings" {
                verify "generate the same Passkey trust entry for either Device Key authentication policy" {
                    use crate::model::{ device_key::LocalAuthentication};
                    let original = ModelProfileFixture::default().build();
                    let expected = format!(
                        "sk-ecdsa-sha2-nistp256@openssh.com {} keyringd Work passkey",
                        encode(&original.passkey().to_ssh_public_key().to_bytes())
                    );
                    for format in [
                        SshSignatureAlgorithm::EcdsaSha2NistP256,
                        SshSignatureAlgorithm::SkEcdsaSha2NistP256,
                        SshSignatureAlgorithm::WebAuthnSkEcdsaSha2NistP256,
                    ] {
                        for authentication in [
                            LocalAuthentication::Required,
                            LocalAuthentication::NotRequired,
                        ] {
                            let profile = ModelProfileFixture {
                                format: format.clone(),
                                authentication,
                                key_seed: 12,
                                ..Default::default()
                            }
                            .build();
                            let entry = SshPasskeyTrustEntry::from_profile(&profile);
                            assert_eq!(entry.as_str(), expected);
                            assert_eq!(entry.to_string(), expected);
                            assert_eq!(AsRef::<str>::as_ref(&entry), expected);
                        }
                    }
                }
            }
        }
    }
}
