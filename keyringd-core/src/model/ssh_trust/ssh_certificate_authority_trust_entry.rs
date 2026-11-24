use crate::model::Profile;
use crate::util::base64::encode;
use keyringd_ssh_agent_protocol::SshSignatureAlgorithm;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SshCertificateAuthorityTrustEntry(String);

impl SshCertificateAuthorityTrustEntry {
    pub fn from_profile(profile: &Profile) -> Self {
        let identity = profile.passkey();
        let blob = identity.to_ssh_public_key();
        Self(format!(
            "cert-authority{},principals=\"{}\" sk-ecdsa-sha2-nistp256@openssh.com {} keyringd-ca",
            if profile.device_key().signature_algorithm()
                == SshSignatureAlgorithm::EcdsaSha2NistP256
            {
                ""
            } else if profile.device_key().local_authentication()
                == crate::model::device_key::LocalAuthentication::Required
            {
                ",verify-required"
            } else {
                ",no-touch-required"
            },
            identity.ssh_certificate_principal(),
            encode(&blob.to_bytes())
        ))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for SshCertificateAuthorityTrustEntry {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl AsRef<str> for SshCertificateAuthorityTrustEntry {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::ModelProfileFixture;

    keyringd_testkit::scenario! {
        group "SshCertificateAuthorityTrustEntry::from_profile" {
            case "Encodes the Passkey CA and principal with the Device Key verification policy" {
                verify "generate the authorized_keys CA line with the expected principal and verification option" {
                    use crate::model::{ device_key::LocalAuthentication};
                    for (format, authentication, option) in [
                        (
                            SshSignatureAlgorithm::EcdsaSha2NistP256,
                            LocalAuthentication::NotRequired,
                            "",
                        ),
                        (SshSignatureAlgorithm::EcdsaSha2NistP256, LocalAuthentication::Required, ""),
                        (
                            SshSignatureAlgorithm::SkEcdsaSha2NistP256,
                            LocalAuthentication::NotRequired,
                            ",no-touch-required",
                        ),
                        (
                            SshSignatureAlgorithm::SkEcdsaSha2NistP256,
                            LocalAuthentication::Required,
                            ",verify-required",
                        ),
                        (
                            SshSignatureAlgorithm::WebAuthnSkEcdsaSha2NistP256,
                            LocalAuthentication::NotRequired,
                            ",no-touch-required",
                        ),
                        (
                            SshSignatureAlgorithm::WebAuthnSkEcdsaSha2NistP256,
                            LocalAuthentication::Required,
                            ",verify-required",
                        ),
                    ] {
                        let profile = ModelProfileFixture {
                            format: format.clone(),
                            authentication,
                            ..Default::default()
                        }
                        .build();
                        let blob = profile.passkey().to_ssh_public_key();
                        let principal = profile.passkey().ssh_certificate_principal();
                        let expected = format!(
                            "cert-authority{option},principals=\"{principal}\" sk-ecdsa-sha2-nistp256@openssh.com {} keyringd-ca",
                            encode(&blob.to_bytes())
                        );
                        let entry = SshCertificateAuthorityTrustEntry::from_profile(&profile);
                        assert_eq!(entry.as_str(), expected);
                        assert_eq!(entry.to_string(), expected);
                        assert_eq!(AsRef::<str>::as_ref(&entry), expected);
                    }
                }
            }
        }
    }
}
