use std::path::{Path, PathBuf};
use std::process::ExitStatus;

use base64::{Engine, engine::general_purpose::STANDARD_NO_PAD};
use keyringd_ssh_agent_protocol::{SshCertificate, SshPublicKey};
use sha2::{Digest, Sha256};

use crate::agent::AgentIdentity;

pub struct SshCommandResult {
    pub status: ExitStatus,
    pub stdout: String,
    pub stderr: String,
    pub server_log: String,
    /// Some clients select a public IdentityFile instead of offering Agent comments.
    pub identity_file: Option<PathBuf>,
}

impl SshCommandResult {
    pub fn assert_success(&self) {
        assert!(
            self.status.success(),
            "SSH failed: {}\nsshd: {}",
            self.stderr,
            self.server_log
        );
        assert!(
            self.server_log.contains("Accepted publickey"),
            "{}",
            self.server_log
        );
    }

    pub fn assert_authenticated(&self, identity: &AgentIdentity) {
        self.assert_success();
        let accepted = self
            .stderr
            .lines()
            .find(|line| line.contains("Server accepts key:"))
            .unwrap_or_else(|| {
                panic!(
                    "OpenSSH did not report an accepted identity: {}",
                    self.stderr
                )
            });
        Self::assert_accepted_identity(accepted, identity, self.identity_file.as_deref());
    }

    fn assert_accepted_identity(
        accepted: &str,
        identity: &AgentIdentity,
        identity_file: Option<&Path>,
    ) {
        let details = accepted.split_once("Server accepts key: ").unwrap().1;
        let (label_and_kind, fingerprint_and_source) = details.rsplit_once(" SHA256:").unwrap();
        let (label, kind) = label_and_kind.rsplit_once(' ').unwrap();
        let fingerprint = fingerprint_and_source.split_whitespace().next().unwrap();
        let expected_kind = match identity.algorithm.as_str() {
            "ecdsa-sha2-nistp256" => "ECDSA",
            "ecdsa-sha2-nistp256-cert-v01@openssh.com" => "ECDSA-CERT",
            "sk-ecdsa-sha2-nistp256@openssh.com" => "ECDSA-SK",
            "sk-ecdsa-sha2-nistp256-cert-v01@openssh.com" => "ECDSA-SK-CERT",
            other => panic!("unsupported SSH identity algorithm: {other}"),
        };
        assert_eq!(kind, expected_kind, "wrong accepted key type: {accepted}");
        // OpenSSH fingerprints a certificate's subject key, excluding its nonce,
        // signature, principals and other certificate metadata.
        let fingerprint_key = if identity.algorithm.contains("-cert-v01@") {
            SshCertificate::from_bytes(&identity.public_key)
                .unwrap()
                .public_key()
        } else {
            SshPublicKey::from_bytes(&identity.public_key).unwrap()
        };
        let expected_fingerprint =
            STANDARD_NO_PAD.encode(Sha256::digest(fingerprint_key.to_bytes()));
        assert_eq!(
            fingerprint, expected_fingerprint,
            "wrong accepted public key: {accepted}"
        );
        if let Some(path) = identity_file {
            let path = path.to_string_lossy();
            assert!(
                label == path
                    || label == path.replace('\\', "\\\\")
                    || label == path.replace('\\', "/"),
                "the accepted key was not the explicitly selected public identity: {accepted}"
            );
        } else {
            assert_eq!(
                label, identity.comment,
                "wrong accepted identity: {accepted}"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use keyringd_ssh_agent_protocol::{
        SshCertificateBody, SshCertificateFields, SshCertificateType, SshSignature,
    };

    const KEY: &str = "ecdsa-sha2-nistp256 AAAAE2VjZHNhLXNoYTItbmlzdHAyNTYAAAAIbmlzdHAyNTYAAABBBIiWPUGrz9FTgZDBjWzRNAOqsX3P16Y5Qb6/VBKhFqDyroSy9MR5wghiZYPFKiI0YIdOamg1IKlB2f9WcRtfnQA= test key";
    const FINGERPRINT: &str = "jFDLpfVadL4IataTSqkkbMzE+N6ltOuXKll80OctFrs";

    keyringd_testkit::scenario! {
        group "SshCommandResult::assert_success" {
            case "Connection success accepts any command output and rejects failed authentication" {
                verify "use successful authentication to identify a connection regardless of command output" {
                    std::thread::spawn(|| {
                        #[cfg(unix)]
                        use std::os::unix::process::ExitStatusExt;
                        #[cfg(windows)]
                        use std::os::windows::process::ExitStatusExt;

                        let result = |status, stdout: &str, server_log: &str| SshCommandResult {
                            status: ExitStatus::from_raw(status),
                            stdout: stdout.into(),
                            stderr: String::new(),
                            server_log: server_log.into(),
                            identity_file: None,
                        };
                        for stdout in [
                            "keyringd-ssh-ok\n",
                            "keyringd-ssh-ok\r\n",
                            "first-connection",
                            "",
                        ] {
                            result(0, stdout, "Accepted publickey").assert_success();
                        }
                        for output in [
                            result(256, "keyringd-ssh-ok\n", "Accepted publickey"),
                            result(0, "keyringd-ssh-ok\n", "Failed publickey"),
                        ] {
                            assert!(std::panic::catch_unwind(|| output.assert_success()).is_err());
                        }
                    })
                    .join()
                    .unwrap();
                }
            }
        }
    }

    fn certificate_identity() -> AgentIdentity {
        let key = AgentIdentity::from_openssh(KEY).unwrap();
        let public_key = SshPublicKey::from_bytes(&key.public_key).unwrap();
        let certificate = SshCertificateBody::from_public_key(
            vec![1; 32],
            public_key.clone(),
            SshCertificateFields {
                serial: 1,
                certificate_type: SshCertificateType::User,
                key_id: b"test certificate".to_vec(),
                valid_principals: vec![b"test".to_vec()],
                valid_after: 0,
                valid_before: u64::MAX,
                critical_options: vec![],
                extensions: vec![],
                reserved: vec![],
                signature_key: public_key,
            },
        )
        .prepare()
        .unwrap()
        .with_signature(SshSignature::Unsupported {
            algorithm: "test-signature".into(),
            data: vec![1],
        });
        AgentIdentity {
            algorithm: "ecdsa-sha2-nistp256-cert-v01@openssh.com".into(),
            public_key: certificate.to_bytes(),
            comment: "test certificate".into(),
        }
    }

    keyringd_testkit::scenario! {
        group "SshCommandResult::assert_accepted_identity" {
            case "Accepted identity handles windows log escaping and certificate subject fingerprint" {
                verify "extract the Windows-escaped identity path and verify the certificate's subject fingerprint" {
                    std::thread::spawn(|| {
                        let identity = certificate_identity();
                        let path = Path::new(r"C:\Users\test user\Temp\identity.pub");
                        for label in [
                            path.to_string_lossy().into_owned(),
                            r"C:\\Users\\test user\\Temp\\identity.pub".into(),
                            "C:/Users/test user/Temp/identity.pub".into(),
                        ] {
                            let line = format!(
                                "debug1: Server accepts key: {label} ECDSA-CERT SHA256:{FINGERPRINT} explicit agent"
                            );
                            SshCommandResult::assert_accepted_identity(&line, &identity, Some(path));
                        }
                    })
                    .join()
                    .unwrap();
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "SshCommandResult::assert_accepted_identity" {
            case "Accepted identity rejects other files keys and certificate types" {
                verify "reject accepted-key log entries for an unrelated path, fingerprint, or key type" {
                    std::thread::spawn(|| {
                        let identity = certificate_identity();
                        let path = Path::new(r"C:\Temp\identity.pub");
                        for (label, kind, fingerprint) in [
                            (r"C:\Temp\identity.pub.other", "ECDSA-CERT", FINGERPRINT),
                            (r"C:\Temp\identity.pub", "ECDSA-CERT", "another-public-key"),
                            (r"C:\Temp\identity.pub", "ECDSA", FINGERPRINT),
                        ] {
                            let line = format!(
                                "debug1: Server accepts key: {label} {kind} SHA256:{fingerprint} explicit agent"
                            );
                            assert!(
                                std::panic::catch_unwind(|| SshCommandResult::assert_accepted_identity(
                                    &line,
                                    &identity,
                                    Some(path)
                                ))
                                .is_err()
                            );
                        }
                    })
                    .join()
                    .unwrap();
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "SshCommandResult::assert_accepted_identity" {
            case "Agent identity requires the complete comment and public key" {
                verify "require both the full comment and the expected public key in agent output" {
                    std::thread::spawn(|| {
                        let identity = AgentIdentity::from_openssh(KEY).unwrap();
                        let line = format!("debug1: Server accepts key: test key ECDSA SHA256:{FINGERPRINT} agent");
                        SshCommandResult::assert_accepted_identity(&line, &identity, None);
                        let wrong = line.replace("test key ECDSA", "other test key ECDSA");
                        assert!(
                            std::panic::catch_unwind(|| SshCommandResult::assert_accepted_identity(
                                &wrong, &identity, None
                            ))
                            .is_err()
                        );
                    })
                    .join()
                    .unwrap();
                }
            }
        }
    }
}
