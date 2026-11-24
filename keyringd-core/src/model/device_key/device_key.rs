use crate::model::device_key::{DevicePrivateKeyHandle, LocalAuthentication};
use keyringd_ssh_agent_protocol::SshSignatureAlgorithm;
use keyringd_ssh_agent_protocol::{SshCertificate, SshCertificateBody, SshPublicKeyBlob};

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeviceKey {
    local_authentication: LocalAuthentication,
    signature_algorithm: SshSignatureAlgorithm,
    #[serde(with = "certificate_serde")]
    certificate: SshCertificate,
    handle: DevicePrivateKeyHandle,
}

impl DeviceKey {
    pub(crate) fn new(
        handle: DevicePrivateKeyHandle,
        certificate: SshCertificate,
        local_authentication: LocalAuthentication,
        signature_algorithm: SshSignatureAlgorithm,
    ) -> Result<Self, &'static str> {
        Self::validate_signature_algorithm(&signature_algorithm)?;
        let matches_subject = matches!(
            (certificate.body(), &signature_algorithm),
            (
                SshCertificateBody::EcdsaP256(_),
                SshSignatureAlgorithm::EcdsaSha2NistP256
            ) | (
                SshCertificateBody::SkEcdsaP256(_),
                SshSignatureAlgorithm::SkEcdsaSha2NistP256
                    | SshSignatureAlgorithm::WebAuthnSkEcdsaSha2NistP256
            )
        );
        if !matches_subject {
            return Err(
                "device key signature algorithm does not match the certificate subject key",
            );
        }
        if let SshCertificateBody::SkEcdsaP256(body) = certificate.body() {
            std::str::from_utf8(&body.application)
                .map_err(|_| "device key application must be a UTF-8 RP ID")?;
        }
        Ok(Self {
            local_authentication,
            signature_algorithm,
            certificate,
            handle,
        })
    }

    pub(crate) fn validate_signature_algorithm(
        algorithm: &SshSignatureAlgorithm,
    ) -> Result<(), &'static str> {
        match algorithm {
            SshSignatureAlgorithm::EcdsaSha2NistP256
            | SshSignatureAlgorithm::SkEcdsaSha2NistP256
            | SshSignatureAlgorithm::WebAuthnSkEcdsaSha2NistP256 => Ok(()),
            SshSignatureAlgorithm::Unsupported(_) => {
                Err("unsupported Device Key signature algorithm")
            }
        }
    }

    pub fn local_authentication(&self) -> LocalAuthentication {
        self.local_authentication
    }

    pub fn signature_algorithm(&self) -> SshSignatureAlgorithm {
        self.signature_algorithm.clone()
    }

    pub fn certificate(&self) -> &SshCertificate {
        &self.certificate
    }

    pub(crate) fn relying_party_id(&self) -> Option<&str> {
        match self.certificate.body() {
            SshCertificateBody::EcdsaP256(_) => None,
            SshCertificateBody::SkEcdsaP256(body) => {
                Some(std::str::from_utf8(&body.application).expect("DeviceKey validates its RP ID"))
            }
        }
    }

    pub fn handle(&self) -> &DevicePrivateKeyHandle {
        &self.handle
    }

    pub(crate) fn direct_identity(&self) -> SshPublicKeyBlob {
        self.certificate.public_key().into_key_blob()
    }
}

impl<'de> serde::Deserialize<'de> for DeviceKey {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(serde::Deserialize)]
        #[serde(rename_all = "camelCase", deny_unknown_fields)]
        struct Fields {
            local_authentication: LocalAuthentication,
            signature_algorithm: SshSignatureAlgorithm,
            #[serde(with = "certificate_serde")]
            certificate: SshCertificate,
            handle: DevicePrivateKeyHandle,
        }
        let fields = Fields::deserialize(deserializer)?;
        Self::new(
            fields.handle,
            fields.certificate,
            fields.local_authentication,
            fields.signature_algorithm,
        )
        .map_err(serde::de::Error::custom)
    }
}

mod certificate_serde {
    use super::SshCertificate;
    pub fn serialize<S: serde::Serializer>(
        value: &SshCertificate,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        crate::util::base64url_serde::serialize(value.to_bytes().as_slice(), serializer)
    }
    pub fn deserialize<'de, D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> Result<SshCertificate, D::Error> {
        let bytes = crate::util::base64url_serde::deserialize(deserializer)?;
        SshCertificate::from_bytes(&bytes).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    keyringd_testkit::scenario! {
        group "DeviceKey::deserialize" {
            case "Accepts only signature algorithms compatible with the certificate subject" {
                why "Stored algorithms must not make the agent return a signature incompatible with its advertised subject key."
                verify "construction and restoration enforce the same compatibility matrix independently of the CA signature" {
                    use crate::test_support::ModelProfileFixture;
                    for subject_algorithm in [
                        SshSignatureAlgorithm::EcdsaSha2NistP256,
                        SshSignatureAlgorithm::SkEcdsaSha2NistP256,
                    ] {
                        let profile = ModelProfileFixture {
                            format: subject_algorithm.clone(),
                            ..Default::default()
                        }.build();
                        let key = profile.device_key();
                        for algorithm in [
                            SshSignatureAlgorithm::EcdsaSha2NistP256,
                            SshSignatureAlgorithm::SkEcdsaSha2NistP256,
                            SshSignatureAlgorithm::WebAuthnSkEcdsaSha2NistP256,
                            SshSignatureAlgorithm::Unsupported("unknown-signature@example.com".into()),
                        ] {
                            let expected = match subject_algorithm {
                                SshSignatureAlgorithm::EcdsaSha2NistP256 => algorithm == SshSignatureAlgorithm::EcdsaSha2NistP256,
                                _ => matches!(algorithm, SshSignatureAlgorithm::SkEcdsaSha2NistP256 | SshSignatureAlgorithm::WebAuthnSkEcdsaSha2NistP256),
                            };
                            assert_eq!(DeviceKey::new(
                                key.handle().clone(), key.certificate().clone(),
                                key.local_authentication(), algorithm.clone(),
                            ).is_ok(), expected, "{subject_algorithm:?} {algorithm:?}");
                            let mut stored = serde_json::to_value(key).unwrap();
                            stored["signatureAlgorithm"] = serde_json::to_value(&algorithm).unwrap();
                            assert_eq!(stored["signatureAlgorithm"], algorithm.as_str());
                            assert_eq!(serde_json::from_value::<DeviceKey>(stored).is_ok(), expected,
                                "{subject_algorithm:?} {algorithm:?}");
                        }
                    }
                }
            }
            case "Accepts RP IDs from the certificate body and rejects non-UTF-8 applications" {
                verify "construction and restoration enforce the CTAP string representation without requiring the product default" {
                    let profile = crate::test_support::ModelProfileFixture { format: SshSignatureAlgorithm::SkEcdsaSha2NistP256, ..Default::default() }.build();
                    let original = profile.device_key();
                    for (application, expected) in [(b"ssh:other".as_slice(), true), (b"ssh:\xff".as_slice(), false)] {
                        let mut body = original.certificate().body().clone();
                        let SshCertificateBody::SkEcdsaP256(subject) = &mut body else { unreachable!() };
                        subject.application = application.to_vec();
                        let certificate = body.prepare().unwrap().with_signature(original.certificate().signature().clone());
                        assert_eq!(DeviceKey::new(original.handle().clone(), certificate.clone(), original.local_authentication(), original.signature_algorithm()).is_ok(), expected);
                        let mut encoded = serde_json::to_value(original).unwrap();
                        use base64::Engine;
                        encoded["certificate"] = serde_json::Value::String(base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(certificate.to_bytes()));
                        assert_eq!(serde_json::from_value::<DeviceKey>(encoded).is_ok(), expected);
                    }
                }
            }
            case "Round-trips key material and its authentication and signature settings" {
                verify "restore the same handle, certificate, local authentication policy, and signature algorithm" {
                    let key = crate::test_support::ModelProfileFixture::default().build().device_key().clone();
                    let encoded = serde_json::to_value(&key).unwrap();
                    let restored: DeviceKey = serde_json::from_value(encoded).unwrap();
                    assert_eq!(restored.handle().as_ref(), key.handle().as_ref());
                    assert_eq!(restored.certificate().to_key_blob(), key.certificate().to_key_blob());
                    assert_eq!(restored.local_authentication(), key.local_authentication());
                    assert_eq!(restored.signature_algorithm(), key.signature_algorithm());
                }
            }

            case "Rejects malformed certificate encodings when restoring a Device Key" {
                verify "return a deserialization error for an invalid certificate" {
                    use crate::test_support::ModelProfileFixture;
                    let key = ModelProfileFixture::default().build().device_key().clone();
                    for certificate in ["!", "AQ"] {
                        let mut encoded = serde_json::to_value(&key).unwrap();
                        encoded["certificate"] = certificate.into();
                        assert!(serde_json::from_value::<DeviceKey>(encoded).is_err());
                    }
                }
            }
        }
    }
}
