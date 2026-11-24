use super::{
    EcdsaP256CertificateBody, SkEcdsaP256CertificateBody, SshCertificateBody, SshCertificateFields,
};
use crate::protocol::codec::{SshReader, SshWriter};
use crate::{
    SshAgentCodecError, SshCertificateCriticalOption, SshCertificateExtension, SshCertificateType,
    SshEllipticCurve, SshKeyAlgorithm, SshPublicKey, SshPublicKeyBlob, SshSignature,
};
use std::time::{SystemTime, UNIX_EPOCH};

/// Structurally validated signed certificate. Parsing does not verify its signature.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SshCertificate {
    body: SshCertificateBody,
    signature: SshSignature,
}

impl SshCertificate {
    pub(super) fn new(body: SshCertificateBody, signature: SshSignature) -> Self {
        Self { body, signature }
    }

    /// The public key certified by this certificate, without certificate metadata.
    pub fn public_key(&self) -> SshPublicKey {
        self.body
            .public_key()
            .expect("validated certificate subject fields")
    }

    pub fn body(&self) -> &SshCertificateBody {
        &self.body
    }
    /// Algorithm of the certificate itself, distinct from the CA's signature algorithm.
    pub fn algorithm(&self) -> SshKeyAlgorithm {
        self.body.algorithm()
    }
    pub fn nonce(&self) -> &[u8] {
        self.body.nonce()
    }
    pub fn serial(&self) -> u64 {
        self.body.fields().serial
    }
    pub fn certificate_type(&self) -> SshCertificateType {
        self.body.fields().certificate_type
    }
    pub fn key_id(&self) -> &[u8] {
        &self.body.fields().key_id
    }
    pub fn valid_principals(&self) -> &[Vec<u8>] {
        &self.body.fields().valid_principals
    }
    pub fn valid_after(&self) -> u64 {
        self.body.fields().valid_after
    }
    pub fn valid_before(&self) -> u64 {
        self.body.fields().valid_before
    }
    pub fn critical_options(&self) -> &[SshCertificateCriticalOption] {
        &self.body.fields().critical_options
    }
    pub fn extensions(&self) -> &[SshCertificateExtension] {
        &self.body.fields().extensions
    }
    pub fn reserved(&self) -> &[u8] {
        &self.body.fields().reserved
    }
    pub fn signature_key(&self) -> &SshPublicKey {
        &self.body.fields().signature_key
    }
    pub fn signature(&self) -> &SshSignature {
        &self.signature
    }

    /// Wrap the parsed certificate as a public key blob.
    pub fn to_key_blob(&self) -> SshPublicKeyBlob {
        SshPublicKeyBlob::from_bytes(&self.to_bytes())
            .expect("encoded certificates have an algorithm")
    }

    /// Encode the certificate fields and signature in SSH wire format.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut body = self.body.encode();
        let mut signature_writer = SshWriter::new();
        signature_writer.write_string(&self.signature.to_bytes());
        body.extend(signature_writer.into_bytes());
        body
    }

    pub fn is_within_validity_period(&self, now: SystemTime) -> bool {
        let Ok(now) = now.duration_since(UNIX_EPOCH) else {
            return false;
        };
        self.body.fields().valid_after <= now.as_secs()
            && now.as_secs() < self.body.fields().valid_before
    }
    /// Parse the certificate's type-specific fields in their wire order.
    pub fn from_bytes(data: &[u8]) -> Result<Self, SshAgentCodecError> {
        let mut reader = SshReader::new(data);
        let key_type = read_string(&mut reader, "key type")?;
        let sk = match key_type {
            b"ecdsa-sha2-nistp256-cert-v01@openssh.com" => false,
            b"sk-ecdsa-sha2-nistp256-cert-v01@openssh.com" => true,
            _ => return Err(SshAgentCodecError::Unsupported { field: "key type" }),
        };
        let nonce = read_string(&mut reader, "nonce")?.to_vec();
        let curve = read_string(&mut reader, "curve name")?;
        if SshEllipticCurve::from_bytes(curve) != Some(SshEllipticCurve::NistP256) {
            return Err(SshAgentCodecError::Unsupported {
                field: "curve name",
            });
        }
        let q = read_string(&mut reader, "public key")?.to_vec();
        let application = if sk {
            Some(read_string(&mut reader, "application")?.to_vec())
        } else {
            None
        };
        let fields = SshCertificateFields::read(&mut reader)?;
        let signature = SshSignature::from_bytes(read_string(&mut reader, "signature")?)
            .map_err(|_| SshAgentCodecError::InvalidSignature)?;
        if !reader.remaining().is_empty() {
            return Err(SshAgentCodecError::TrailingData {
                trailing_bytes: reader.remaining().len(),
            });
        }
        let body = if let Some(application) = application {
            SshCertificateBody::SkEcdsaP256(SkEcdsaP256CertificateBody {
                nonce,
                q,
                application,
                fields,
            })
        } else {
            SshCertificateBody::EcdsaP256(EcdsaP256CertificateBody { nonce, q, fields })
        };
        body.validate()?;
        Ok(Self::new(body, signature))
    }
}

fn read_string<'a>(
    reader: &mut SshReader<'a>,
    field: &'static str,
) -> Result<&'a [u8], SshAgentCodecError> {
    reader
        .read_string()
        .map_err(|_| SshAgentCodecError::Truncated { field })
}

#[cfg(test)]
mod tests {
    use super::*;
    use p256::elliptic_curve::sec1::ToSec1Point;

    fn wire_certificate(sk: bool, q: &[u8], ca: &[u8]) -> Vec<u8> {
        let mut writer = SshWriter::new();
        writer.write_string(if sk {
            b"sk-ecdsa-sha2-nistp256-cert-v01@openssh.com"
        } else {
            b"ecdsa-sha2-nistp256-cert-v01@openssh.com"
        });
        writer.write_string(&[7, 8]);
        writer.write_string(b"nistp256");
        writer.write_string(q);
        if sk {
            writer.write_string(b"ssh:keyringd");
        }
        writer.write_u64(17);
        writer.write_u32(1);
        writer.write_string(b"layout");
        let mut principals = SshWriter::new();
        principals.write_string(b"user");
        writer.write_string(&principals.into_bytes());
        writer.write_u64(10);
        writer.write_u64(20);
        writer.write_string(b""); // critical options
        writer.write_string(b""); // extensions
        writer.write_string(b"reserved");
        writer.write_string(ca);
        writer.write_string(&dummy_signature().to_bytes());
        writer.into_bytes()
    }

    fn dummy_signature() -> SshSignature {
        let mut sig_bytes = [0u8; 64];
        sig_bytes[..32].fill(0x01);
        sig_bytes[32..].fill(0x02);
        crate::SkEcdsaP256SshSignature::new(
            crate::EcdsaP256SshSignature::new(&sig_bytes[..32], &sig_bytes[32..]),
            0x01,
            1,
        )
        .into_signature()
    }

    keyringd_testkit::scenario! {
            group "SshCertificate::from_bytes" {
            case "Decodes inline subject fields and the separate CA key blob in wire order" {
                why "A certificate subject is curve/Q/application, while signature key is a nested public key blob."
                verify "both certificate types expose the exact fields encoded by a wire-level fixture" {
                    let q = p256::SecretKey::from_slice(&[1; 32]).unwrap().public_key().to_sec1_point(true);
                    let mut ca_writer = SshWriter::new();
                    ca_writer.write_string(b"ecdsa-sha2-nistp256");
                    ca_writer.write_string(b"nistp256");
                    ca_writer.write_string(q.as_bytes());
                    let ca = ca_writer.into_bytes();
                    for sk in [false, true] {
                        let encoded = wire_certificate(sk, q.as_bytes(), &ca);
                        let parsed = SshCertificate::from_bytes(&encoded).unwrap();
                        match parsed.body() {
                            SshCertificateBody::EcdsaP256(body) => {
                                assert!(!sk);
                                assert_eq!(body.nonce, [7, 8]);
                                assert_eq!(body.q, q.as_bytes());
                            }
                            SshCertificateBody::SkEcdsaP256(body) => {
                                assert!(sk);
                                assert_eq!(body.nonce, [7, 8]);
                                assert_eq!(body.q, q.as_bytes());
                                assert_eq!(body.application, b"ssh:keyringd");
                            }
                        }
                        assert_eq!(parsed.serial(), 17);
                        assert_eq!(parsed.certificate_type(), SshCertificateType::User);
                        assert_eq!(parsed.key_id(), b"layout");
                        assert_eq!(parsed.valid_principals(), &[b"user".to_vec()]);
                        assert_eq!((parsed.valid_after(), parsed.valid_before()), (10, 20));
                        assert_eq!(parsed.reserved(), b"reserved");
                        assert_eq!(parsed.signature_key().to_bytes(), ca);
                        assert_eq!(parsed.to_bytes(), encoded);
                    }
                }
            }
            case "Rejects unsupported CA key formats" {
                verify "opaque formats at the agent boundary cannot enter the typed certificate CA field" {
                    let q = p256::SecretKey::from_slice(&[1; 32]).unwrap().public_key().to_sec1_point(false);
                    for algorithm in ["ssh-ed25519", "ssh-ed25519-cert-v01@openssh.com"] {
                        let mut ca = SshWriter::new();
                        ca.write_string(algorithm.as_bytes());
                        ca.write_string(&[9; 32]);
                        let ca = ca.into_bytes();
                        for sk in [false, true] {
                            let encoded = wire_certificate(sk, q.as_bytes(), &ca);
                            assert!(matches!(SshCertificate::from_bytes(&encoded), Err(SshAgentCodecError::Unsupported { field: "public key algorithm" })));
                        }
                    }
                }
            }
            case "Rejects a certificate in the CA public key field" {
                why "The signature key field contains a standalone CA key; certificates cannot be chained."
                verify "certificate blobs are accepted at the agent boundary but rejected as CA public keys" {
                    let ca_certificate = certificate().to_bytes();
                    assert_eq!(SshPublicKeyBlob::from_bytes(&ca_certificate).unwrap().as_bytes(), ca_certificate);
                    let q = p256::SecretKey::from_slice(&[1; 32]).unwrap().public_key().to_sec1_point(false);
                    for sk in [false, true] {
                        let encoded = wire_certificate(sk, q.as_bytes(), &ca_certificate);
                        assert!(matches!(SshCertificate::from_bytes(&encoded), Err(SshAgentCodecError::Unsupported { field: "public key algorithm" })));
                    }
                }
            }
                case "Preserves CA signatures when structured certificate keys are re-encoded" {
                    why "Normalizing a compressed subject or CA point changes the signed certificate bytes."
                    verify "ordinary and SK subjects retain the original bytes and a verifiable CA signature" {
                        use p256::ecdsa::signature::{Signer, Verifier};
                        let ca = p256::ecdsa::SigningKey::from_slice(&[9; 32]).unwrap();
                        let ca_point = ca.verifying_key().to_sec1_point(true);
                        let subject = p256::SecretKey::from_slice(&[1; 32]).unwrap().public_key();
                        for compressed in [false, true] {
                            let q = subject.to_sec1_point(compressed);
                            for public_key in [
                                SshPublicKey::from_ecdsa_pubkey(q.as_bytes()).unwrap(),
                                SshPublicKey::from_sk_ecdsa_pubkey(q.as_bytes(), "ssh:keyringd").unwrap(),
                            ] {
                                let mut body = certificate().body;
                                body = SshCertificateBody::from_public_key(body.nonce().to_vec(), public_key.clone(), body.fields().clone());
                                body.fields_mut().signature_key = SshPublicKey::from_ecdsa_pubkey(ca_point.as_bytes()).unwrap();
                                let request = body.prepare().unwrap();
                                let signature: p256::ecdsa::Signature = ca.sign(request.signing_bytes());
                                let raw = signature.to_bytes();
                                let certificate = request.with_signature(
                                    crate::EcdsaP256SshSignature::new(&raw[..32], &raw[32..]).into_signature(),
                                );
                                let encoded = certificate.to_bytes();
                                let decoded = SshPublicKeyBlob::from_bytes(&encoded).unwrap();
                                let parsed = SshCertificate::from_bytes(decoded.as_bytes()).unwrap();
                                assert_eq!(parsed.public_key(), public_key);
                                assert_eq!(parsed.signature(), certificate.signature());
                                ca.verifying_key().verify(&parsed.body.encode(), &signature).unwrap();
                                assert_eq!(decoded.to_bytes(), encoded);
                            }
                        }
                    }
                }
                case "ECDSA P-256 Round-trip Convert SSH Certificate" {
                    spec "https://github.com/openssh/openssh-portable/blob/V_9_9_P2/PROTOCOL.certkeys#L104-L121"
                    given "Prepare ECDSA P-256 User Certificate with all fields such as Serial, validity period and Principal" {
                        let public_key = p256::SecretKey::from_slice(&[1; 32])
                            .unwrap()
                            .public_key()
                            .to_sec1_point(false);
                        let now = SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap()
                            .as_secs();
                        let valid_after = now - 300;
                        let valid_before = now + 86400;
                        let signature_key =
                            SshPublicKey::from_sk_ecdsa_pubkey(public_key.as_bytes(), "ssh:").unwrap();
                        let certificate = SshCertificateBody::from_public_key(vec![7; 32], SshPublicKey::from_ecdsa_pubkey(public_key.as_bytes()).unwrap(), SshCertificateFields {
    serial: 1,
    certificate_type: SshCertificateType::User,
    key_id: b"keyringd-tpm".to_vec(),
    valid_principals: vec![b"testuser".to_vec()],
    valid_after,
    valid_before,
    critical_options: vec![SshCertificateCriticalOption::new(
                                b"source-address",
                                b"10.0.0.0/8".to_vec(),
                            )],
    extensions: vec![SshCertificateExtension::empty(b"permit-pty")],
    reserved: b"reserved".to_vec(),
    signature_key,
    })
                        .prepare()
                        .unwrap()
                        .with_signature(dummy_signature());
                    }
                    when "Encode Certificate to SSH Wire format and decode again" {
                        let blob = certificate.to_key_blob();
                        let parsed = SshCertificate::from_bytes(&blob.to_bytes()).unwrap();
                    }
                    then "All Fields match the original value and the Decoder consumes the entire Payload" {
                        assert_eq!(
                            certificate.public_key(),
                            SshPublicKey::from_ecdsa_pubkey(public_key.as_bytes()).unwrap()
                        );
                        assert_eq!(parsed.public_key(), certificate.public_key());
                        assert_eq!(certificate.key_id(), b"keyringd-tpm");
                        assert_eq!(certificate.valid_principals(), vec![b"testuser"]);
                        assert!(matches!(
                            certificate.signature,
                            keyringd_ssh_agent_protocol::SshSignature::SkEcdsa(_)
                        ));
                        assert!(now - certificate.valid_after() >= 300);
                        assert_eq!(parsed.valid_principals(), certificate.valid_principals());
                        assert_eq!(parsed.valid_before(), certificate.valid_before());
                        assert_eq!(parsed.critical_options(), certificate.critical_options());
                        assert_eq!(parsed.extensions(), certificate.extensions());
                        assert_eq!(parsed.reserved(), certificate.reserved());
                        assert_eq!(parsed.signature_key(), certificate.signature_key());
                        assert_eq!(&parsed.signature, &certificate.signature);
                    }
                }
                case "Reject the cut ECDSA P-256 SSH Certificate" {
                    spec "https://github.com/openssh/openssh-portable/blob/V_9_9_P2/PROTOCOL.certkeys#L104-L121"
                    given "Prepare a key type string that exceeds the number of bytes declared length is available" {
                        let blob = vec![0, 0, 0, 64];
                    }
                    when "Parse the value as a certificate" {
                        let parsed = SshCertificate::from_bytes(&blob);
                    }
                    then "Report as a disconnect of the Key Type Field" {
                        assert!(matches!(
                            parsed,
                            Err(SshAgentCodecError::Truncated { field: "key type" })
                        ));
                    }
                }
                case "ECDSA P-256 SSH Certificate Reject Extra Data" {
                    why "Requires the consumption of the certificate blob and does not misidentify consolidated data as a valid single certificate"
                    spec "https://github.com/openssh/openssh-portable/blob/V_9_9_P2/PROTOCOL.certkeys#L104-L121"
                    given "Add one extra BYTE at the end of a valid Certificate Blob" {
                        let certificate = certificate();
                        let mut bytes = certificate.to_bytes();
                        bytes.push(0);
                    }
                    when "Parse Certificate" {
                        let parsed = SshCertificate::from_bytes(&bytes);
                    }
                    then "Report Extra BYTE as Trailing Data" {
                        assert!(matches!(
                            parsed,
                            Err(SshAgentCodecError::TrailingData { trailing_bytes: 1 })
                        ));
                    }
                }
                case "Reject unsupported SSH Certificate Algorithm" {
                    why "Because it does not inadvertently accept the certificate of unsupported Algorithm"
                    spec "https://github.com/openssh/openssh-portable/blob/V_9_9_P2/PROTOCOL.certkeys#L104-L121"
                    given "Prepare a Certificate Blob with Modified Key Type" {
                        let certificate = certificate();
                        let mut bytes = certificate.to_bytes();
                        bytes[4] ^= 1;
                    }
                    when "Parse Certificate" {
                        let parsed = SshCertificate::from_bytes(&bytes);
                    }
                    then "Returns UnsupportedKeyType ERROR without interpreting ECDSA P-256" {
                        assert!(matches!(
                            parsed,
                            Err(SshAgentCodecError::Unsupported { field: "key type" })
                        ));
                    }
                }
            }
        }

    keyringd_testkit::scenario! {
            group "SshCertificate::from_bytes" {
                case "Preserves subject and signed body for both key formats" {
                    verify "recover the same subject key and signing bytes for ECDSA and SK-ECDSA" {
                        let point = p256::SecretKey::from_slice(&[1; 32])
                            .unwrap()
                            .public_key()
                            .to_sec1_point(false);
                        let subjects = [
                            SshPublicKey::from_ecdsa_pubkey(point.as_bytes()).unwrap(),
                            SshPublicKey::from_sk_ecdsa_pubkey(point.as_bytes(), "ssh:keyringd").unwrap(),
                        ];
                        for (subject, expected_algorithm) in subjects.into_iter().zip([
                            SshKeyAlgorithm::EcdsaSha2NistP256Certificate,
                            SshKeyAlgorithm::SkEcdsaSha2NistP256Certificate,
                        ]) {
                            let body = SshCertificateBody::from_public_key(vec![7; 32], subject.clone(), SshCertificateFields {
    serial: 1,
    certificate_type: SshCertificateType::User,
    key_id: b"test".to_vec(),
    valid_principals: vec![b"testuser".to_vec()],
    valid_after: 0,
    valid_before: 3600,
    critical_options: Vec::new(),
    extensions: Vec::new(),
    reserved: Vec::new(),
    signature_key: SshPublicKey::from_sk_ecdsa_pubkey(point.as_bytes(), "ssh:")
                                    .unwrap(),
    })
                            .prepare()
                            .unwrap();
                            let signed_body = body.signing_bytes().to_vec();
                            let certificate = body.with_signature(dummy_signature());
                            assert_eq!(certificate.algorithm(), expected_algorithm);
                            let blob = certificate.to_key_blob();
                            let mut signature_writer = SshWriter::new();
                            signature_writer.write_string(&certificate.signature().to_bytes());
                            let expected = [signed_body, signature_writer.into_bytes()].concat();
                            assert_eq!(blob.to_bytes(), expected);
                            let parsed = SshCertificate::from_bytes(&blob.to_bytes()).unwrap();
                            assert_eq!(parsed.algorithm(), expected_algorithm);
                            assert_eq!(parsed.public_key(), subject);
                            assert_eq!(parsed.to_key_blob(), blob);
                        }
                    }
                }
            }
        }

    fn certificate() -> SshCertificate {
        let public_key = p256::SecretKey::from_slice(&[1; 32])
            .unwrap()
            .public_key()
            .to_sec1_point(false);
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let signature_key =
            SshPublicKey::from_sk_ecdsa_pubkey(public_key.as_bytes(), "ssh:").unwrap();
        SshCertificateBody::from_public_key(
            vec![7; 32],
            SshPublicKey::from_ecdsa_pubkey(public_key.as_bytes()).unwrap(),
            SshCertificateFields {
                serial: 1,
                certificate_type: SshCertificateType::User,
                key_id: b"test".to_vec(),
                valid_principals: vec![b"testuser".to_vec()],
                valid_after: now - 300,
                valid_before: now + 3600,
                critical_options: Vec::new(),
                extensions: Vec::new(),
                reserved: Vec::new(),
                signature_key,
            },
        )
        .prepare()
        .unwrap()
        .with_signature(dummy_signature())
    }
}
