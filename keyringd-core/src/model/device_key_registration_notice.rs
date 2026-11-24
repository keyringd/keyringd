use crate::model::{Profile, ProfileId, device_key::LocalAuthentication};
use keyringd_ssh_agent_protocol::SshSignatureAlgorithm;

/// Explains a required server registration change; does not perform the update.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeviceKeyRegistrationNotice {
    pub profile_id: ProfileId,
    pub message: String,
}

impl DeviceKeyRegistrationNotice {
    pub fn between(previous: &Profile, current: &Profile) -> Option<Self> {
        let public_key_changed = previous.device_key().certificate().public_key()
            != current.device_key().certificate().public_key();
        let verification_changed = previous.device_key().local_authentication()
            != current.device_key().local_authentication();
        let mut messages = Vec::new();
        if verification_changed
            && current.device_key().signature_algorithm()
                != SshSignatureAlgorithm::EcdsaSha2NistP256
        {
            messages.push(match current.device_key().local_authentication() {
                LocalAuthentication::Required =>
                    "Local authentication is now enabled. Changing this setting does not update the server's verification requirements. Replace the registered public-key entry with the updated entry below so the server requires verification too.",
                LocalAuthentication::NotRequired =>
                    "Local authentication is now disabled. If the server's existing registration requires user presence or verification, it will reject connections with this Device Key. Replace the registered public-key entry with the updated entry below.",
            });
        }
        if public_key_changed {
            messages.push("The Device Key's public key has changed. Replace its entry on servers where it is registered directly.");
        }
        if messages.is_empty() {
            return None;
        }
        Some(Self {
            profile_id: current.id().clone(),
            message: messages.join("\n\n"),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::ModelProfileFixture;

    keyringd_testkit::scenario! {
        group "DeviceKeyRegistrationNotice::between" {
            case "Reports key and verification changes independently for each signature format" {
                verify "emit separate notices for changed public keys and changed security-key policies" {
                    for format in [
                        SshSignatureAlgorithm::EcdsaSha2NistP256,
                        SshSignatureAlgorithm::SkEcdsaSha2NistP256,
                        SshSignatureAlgorithm::WebAuthnSkEcdsaSha2NistP256,
                    ] {
                        for (before, after, changed) in [
                            (
                                LocalAuthentication::NotRequired,
                                LocalAuthentication::NotRequired,
                                false,
                            ),
                            (
                                LocalAuthentication::NotRequired,
                                LocalAuthentication::Required,
                                true,
                            ),
                            (
                                LocalAuthentication::Required,
                                LocalAuthentication::NotRequired,
                                true,
                            ),
                            (
                                LocalAuthentication::Required,
                                LocalAuthentication::Required,
                                false,
                            ),
                        ] {
                            for key_changed in [false, true] {
                                let previous = ModelProfileFixture {
                                    format: format.clone(),
                                    authentication: before,
                                    ..Default::default()
                                }
                                .build();
                                let current = ModelProfileFixture {
                                    format: format.clone(),
                                    authentication: after,
                                    key_seed: if key_changed { 12 } else { 11 },
                                    ..Default::default()
                                }
                                .build();
                                let notice = DeviceKeyRegistrationNotice::between(&previous, &current);
                                let policy_notice = changed && format != SshSignatureAlgorithm::EcdsaSha2NistP256;
                                assert_eq!(
                                    notice.is_some(),
                                    key_changed || policy_notice,
                                    "{format:?} {before:?} {after:?}"
                                );
                                if let Some(notice) = notice {
                                    assert_eq!(&notice.profile_id, current.id());
                                    assert_eq!(
                                        notice.message.contains("public key has changed"),
                                        key_changed
                                    );
                                    assert_eq!(
                                        notice.message.contains("Local authentication is now"),
                                        policy_notice
                                    );
                                    if policy_notice {
                                        let expected = if after == LocalAuthentication::Required {
                                            "now enabled"
                                        } else {
                                            "now disabled"
                                        };
                                        assert!(notice.message.contains(expected));
                                    }
                                    assert_eq!(
                                        notice.message.matches("\n\n").count(),
                                        usize::from(key_changed && policy_notice)
                                    );
                                }
                            }
                        }
                    }
                }
            }

            case "Ignores profile metadata and certificate-only changes" {
                verify "omit the registration notice after metadata or certificate validity changes alone" {
                    let original = ModelProfileFixture::default().build();
                    let mut current = ModelProfileFixture {
                        valid_before: 300,
                        ..Default::default()
                    }
                    .build();
                    current.rename("Renamed".into()).unwrap();
                    current.set_enabled(false);
                    assert!(DeviceKeyRegistrationNotice::between(&original, &current).is_none());
                }
            }
        }
    }
}
