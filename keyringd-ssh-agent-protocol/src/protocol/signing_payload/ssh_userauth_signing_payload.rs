use super::SshSigningPayloadError;
use crate::SshPublicKeyBlob;

const SSH_MSG_USERAUTH_REQUEST: u8 = 50;
const SERVICE_SSH_CONNECTION: &[u8] = b"ssh-connection";
const METHOD_PUBLICKEY: &[u8] = b"publickey";
const METHOD_PUBLICKEY_HOSTBOUND: &[u8] = b"publickey-hostbound-v00@openssh.com";

/// Parsed SSH publickey userauth signature data (RFC 4252 Section 7).
///
/// The SSH Agent receives this as the `data` field of SSH_AGENTC_SIGN_REQUEST.
/// Two variants exist:
/// - `"publickey"` (RFC 4252)
/// - `"publickey-hostbound-v00@openssh.com"` (OpenSSH extension, adds host key blob)
#[derive(Debug)]
pub struct SshUserauthSigningPayload<'a> {
    algorithm: &'a [u8],
    key_blob: SshPublicKeyBlob,
    host_key_blob: Option<SshPublicKeyBlob>,
}

impl<'a> SshUserauthSigningPayload<'a> {
    /// Parse SSH publickey userauth signature data.
    ///
    /// Expected format (RFC 4252 Section 7):
    /// ```text
    /// string    session identifier    (must be non-empty)
    /// byte      SSH_MSG_USERAUTH_REQUEST (50)
    /// string    user name
    /// string    service name          (must be "ssh-connection")
    /// string    "publickey" | "publickey-hostbound-v00@openssh.com"
    /// boolean   TRUE                  (must be 1)
    /// string    public key algorithm name
    /// string    public key blob
    /// string    server host key blob  (hostbound only)
    /// ```
    pub fn from_bytes(data: &'a [u8]) -> Result<Self, SshSigningPayloadError> {
        let mut remaining = data;
        let session_id = read_string(&mut remaining).ok_or(SshSigningPayloadError::Truncated {
            field: "session_id",
        })?;
        if session_id.is_empty() {
            return Err(SshSigningPayloadError::EmptySessionId);
        }

        let msg_type = read_u8(&mut remaining)
            .ok_or(SshSigningPayloadError::Truncated { field: "msg_type" })?;
        if msg_type != SSH_MSG_USERAUTH_REQUEST {
            return Err(SshSigningPayloadError::NotUserAuthRequest { actual: msg_type });
        }

        let _user_name = read_string(&mut remaining)
            .ok_or(SshSigningPayloadError::Truncated { field: "user_name" })?;

        let service_name =
            read_string(&mut remaining).ok_or(SshSigningPayloadError::Truncated {
                field: "service_name",
            })?;
        if service_name != SERVICE_SSH_CONNECTION {
            return Err(SshSigningPayloadError::InvalidService);
        }

        let method = read_string(&mut remaining)
            .ok_or(SshSigningPayloadError::Truncated { field: "method" })?;
        let is_hostbound = if method == METHOD_PUBLICKEY {
            false
        } else if method == METHOD_PUBLICKEY_HOSTBOUND {
            true
        } else {
            return Err(SshSigningPayloadError::NotPublicKeyMethod);
        };

        let pk_boolean = read_u8(&mut remaining).ok_or(SshSigningPayloadError::Truncated {
            field: "pk_boolean",
        })?;
        if pk_boolean != 1 {
            return Err(SshSigningPayloadError::PublicKeyBooleanFalse { actual: pk_boolean });
        }

        let algorithm = read_string(&mut remaining)
            .ok_or(SshSigningPayloadError::Truncated { field: "algorithm" })?;
        let public_key_blob =
            read_string(&mut remaining).ok_or(SshSigningPayloadError::Truncated {
                field: "public_key_blob",
            })?;
        let key_blob = SshPublicKeyBlob::from_bytes(public_key_blob).map_err(|source| {
            SshSigningPayloadError::InvalidKeyBlob {
                field: "public_key_blob",
                source,
            }
        })?;
        if key_blob.algorithm().as_bytes() != algorithm {
            return Err(SshSigningPayloadError::KeyAlgorithmMismatch);
        }

        let host_key_blob = if is_hostbound {
            let host_key_bytes =
                read_string(&mut remaining).ok_or(SshSigningPayloadError::Truncated {
                    field: "host_key_blob",
                })?;
            Some(
                SshPublicKeyBlob::from_bytes(host_key_bytes).map_err(|source| {
                    SshSigningPayloadError::InvalidKeyBlob {
                        field: "host_key_blob",
                        source,
                    }
                })?,
            )
        } else {
            None
        };

        if !remaining.is_empty() {
            return Err(SshSigningPayloadError::TrailingData {
                trailing_bytes: remaining.len(),
            });
        }

        Ok(Self {
            algorithm,
            key_blob,
            host_key_blob,
        })
    }

    pub fn algorithm(&self) -> &[u8] {
        self.algorithm
    }

    pub fn key_blob(&self) -> &SshPublicKeyBlob {
        &self.key_blob
    }

    pub fn host_key_blob(&self) -> Option<&SshPublicKeyBlob> {
        self.host_key_blob.as_ref()
    }

    pub fn matches_key_blob(&self, key_blob: &SshPublicKeyBlob) -> bool {
        &self.key_blob == key_blob
    }
}

fn read_u8(bytes: &mut &[u8]) -> Option<u8> {
    let value = *bytes.first()?;
    *bytes = &bytes[1..];
    Some(value)
}

fn read_string<'a>(bytes: &mut &'a [u8]) -> Option<&'a [u8]> {
    let length = u32::from_be_bytes(bytes.get(..4)?.try_into().ok()?) as usize;
    let value = bytes.get(4..4 + length)?;
    *bytes = &bytes[4 + length..];
    Some(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SshPublicKey;
    use crate::{SshKeyAlgorithm, SshPublicKeyBlob};
    use p256::elliptic_curve::sec1::ToSec1Point;

    fn append_string(bytes: &mut Vec<u8>, value: &[u8]) {
        bytes.extend((value.len() as u32).to_be_bytes());
        bytes.extend(value);
    }

    fn mock_public_key_blob() -> SshPublicKeyBlob {
        let public_key = p256::SecretKey::from_slice(&[1; 32])
            .unwrap()
            .public_key()
            .to_sec1_point(false);
        SshPublicKey::from_sk_ecdsa_pubkey(public_key.as_bytes(), "ssh:")
            .unwrap()
            .into_key_blob()
    }

    fn build_publickey_payload(
        session_id: &[u8],
        algorithm: &[u8],
        public_key_blob: &[u8],
    ) -> Vec<u8> {
        let mut buf = Vec::new();
        append_string(&mut buf, session_id);
        buf.push(SSH_MSG_USERAUTH_REQUEST);
        append_string(&mut buf, b"user");
        append_string(&mut buf, SERVICE_SSH_CONNECTION);
        append_string(&mut buf, METHOD_PUBLICKEY);
        buf.push(1);
        append_string(&mut buf, algorithm);
        append_string(&mut buf, public_key_blob);
        buf
    }

    fn build_hostbound_payload(
        session_id: &[u8],
        algorithm: &[u8],
        public_key_blob: &[u8],
        host_key_blob: &[u8],
    ) -> Vec<u8> {
        let mut buf = Vec::new();
        append_string(&mut buf, session_id);
        buf.push(SSH_MSG_USERAUTH_REQUEST);
        append_string(&mut buf, b"user");
        append_string(&mut buf, SERVICE_SSH_CONNECTION);
        append_string(&mut buf, METHOD_PUBLICKEY_HOSTBOUND);
        buf.push(1);
        append_string(&mut buf, algorithm);
        append_string(&mut buf, public_key_blob);
        append_string(&mut buf, host_key_blob);
        buf
    }

    fn valid_payload() -> Vec<u8> {
        let blob = mock_public_key_blob();
        build_publickey_payload(
            b"session-id",
            SshKeyAlgorithm::SkEcdsaSha2NistP256.as_bytes(),
            &blob.to_bytes(),
        )
    }

    // --- success ---

    keyringd_testkit::scenario! {
        group "SshUserauthSigningPayload::from_bytes" {
            case "Analyze Public Key Authentication Payload" {
                spec "https://www.rfc-editor.org/rfc/rfc4252.html#section-7"
                given "Prepare a valid public key authentication Payload" {
                    let blob = mock_public_key_blob();
                    let data = valid_payload();
                }
                when "Parse Payload as SSH UserAuth Signature Data" {
                    let parsed = SshUserauthSigningPayload::from_bytes(&data).unwrap();
                }
                then "The signature algorithm and public key blob are restored" {
                    assert_eq!(
                        parsed.algorithm,
                        SshKeyAlgorithm::SkEcdsaSha2NistP256.as_bytes()
                    );
                    assert_eq!(parsed.key_blob(), &blob);
                }
            }
            case "Analyze Host-Bound Authentication Payload" {
                spec "https://github.com/openssh/openssh-portable/blob/master/PROTOCOL#L321-L338"
                given "Prepare a Host-Bound Authentication Payload with a Host Key Blob" {
                    let blob = mock_public_key_blob();
                    let data = build_hostbound_payload(
                        b"session-id",
                        SshKeyAlgorithm::SkEcdsaSha2NistP256.as_bytes(),
                        &blob.to_bytes(),
                        &blob.to_bytes(),
                    );
                }
                when "Analyze Host-bound Payload" {
                    let parsed = SshUserauthSigningPayload::from_bytes(&data).unwrap();
                }
                then "Accepted as a valid payload with signature algorithm and public key blob" {
                    assert_eq!(
                        parsed.algorithm,
                        SshKeyAlgorithm::SkEcdsaSha2NistP256.as_bytes()
                    );
                    assert_eq!(parsed.key_blob(), &blob);
                    assert_eq!(parsed.host_key_blob(), Some(&blob));
                }
            }
        }
    }

    // --- rejection ---

    keyringd_testkit::scenario! {
        group "SshUserauthSigningPayload::from_bytes" {
            case "Rejects empty signature DATA" {
                spec "https://www.rfc-editor.org/rfc/rfc4252.html#section-7"
                verify "Make sure you reject the empty signature DATA" {
                    assert!(matches!(
                        SshUserauthSigningPayload::from_bytes(&[]),
                        Err(SshSigningPayloadError::Truncated { .. })
                    ));
                }
            }
            case "Rejects invalid SSH string encodings" {
                spec "https://www.rfc-editor.org/rfc/rfc4251.html#section-5"
                spec "https://www.rfc-editor.org/rfc/rfc4252.html#section-7"
                verify "Report that the SSH String of Session ID is cut off in the middle" {
                    assert!(matches!(
                        SshUserauthSigningPayload::from_bytes(b"arbitrary data that is not SSH"),
                        Err(SshSigningPayloadError::Truncated {
                            field: "session_id"
                        })
                    ));
                }
            }
            case "Rejects empty Session ID" {
                why "To tie the public key authentication signature to the actual SSH Transport Session"
                spec "https://www.rfc-editor.org/rfc/rfc4252.html#section-7"
                spec "https://www.rfc-editor.org/rfc/rfc4253.html#section-8"
                given "Prepare an empty public key authentication payload with a Session ID" {
                    let blob = mock_public_key_blob();
                    let data = build_publickey_payload(
                        b"",
                        SshKeyAlgorithm::SkEcdsaSha2NistP256.as_bytes(),
                        &blob.to_bytes(),
                    );
                }
                when "Analyze Payload" {
                    let result = SshUserauthSigningPayload::from_bytes(&data);
                }
                then "Rejected as an empty Session ID" {
                    assert!(matches!(
                        result,
                        Err(SshSigningPayloadError::EmptySessionId)
                    ));
                }
            }
            case "Reject Messages other than User Authentication" {
                spec "https://www.rfc-editor.org/rfc/rfc4252.html#section-7"
                given "Assemble Payload with Message Type not User Authentication" {
                    let blob = mock_public_key_blob();
                    let mut data = Vec::new();
                    append_string(&mut data, b"session-id");
                    data.push(99);
                    append_string(&mut data, b"user");
                    append_string(&mut data, SERVICE_SSH_CONNECTION);
                    append_string(&mut data, METHOD_PUBLICKEY);
                    data.push(1);
                    append_string(&mut data, SshKeyAlgorithm::SkEcdsaSha2NistP256.as_bytes());
                    append_string(&mut data, &blob.to_bytes());
                }
                when "Analyze Payload" {
                    let result = SshUserauthSigningPayload::from_bytes(&data);
                }
                then "Rejected for error with actual message type" {
                    assert!(matches!(
                        result,
                        Err(SshSigningPayloadError::NotUserAuthRequest { actual: 99 })
                    ));
                }
            }
            case "Reject different SSH Service" {
                why "To restrict this parser target to the public key authentication of the SSH Connection Service"
                spec "https://www.rfc-editor.org/rfc/rfc4252.html#section-7"
                given "Assemble a payload with a service name that is not ssh-connection" {
                    let blob = mock_public_key_blob();
                    let mut data = Vec::new();
                    append_string(&mut data, b"session-id");
                    data.push(SSH_MSG_USERAUTH_REQUEST);
                    append_string(&mut data, b"user");
                    append_string(&mut data, b"bad-service");
                    append_string(&mut data, METHOD_PUBLICKEY);
                    data.push(1);
                    append_string(&mut data, SshKeyAlgorithm::SkEcdsaSha2NistP256.as_bytes());
                    append_string(&mut data, &blob.to_bytes());
                }
                when "Analyze Payload" {
                    let result = SshUserauthSigningPayload::from_bytes(&data);
                }
                then "Rejected as an illegal service" {
                    assert!(matches!(
                        result,
                        Err(SshSigningPayloadError::InvalidService)
                    ));
                }
            }
            case "Reject methods other than public key authentication" {
                spec "https://www.rfc-editor.org/rfc/rfc4252.html#section-7"
                given "Assemble a Payload that the Authentication Method is not a PublicKey" {
                    let blob = mock_public_key_blob();
                    let mut data = Vec::new();
                    append_string(&mut data, b"session-id");
                    data.push(SSH_MSG_USERAUTH_REQUEST);
                    append_string(&mut data, b"user");
                    append_string(&mut data, SERVICE_SSH_CONNECTION);
                    append_string(&mut data, b"password");
                    data.push(1);
                    append_string(&mut data, SshKeyAlgorithm::SkEcdsaSha2NistP256.as_bytes());
                    append_string(&mut data, &blob.to_bytes());
                }
                when "Analyze Payload" {
                    let result = SshUserauthSigningPayload::from_bytes(&data);
                }
                then "Rejected because it's not a publickey method" {
                    assert!(matches!(
                        result,
                        Err(SshSigningPayloadError::NotPublicKeyMethod)
                    ));
                }
            }
            case "Public key signature flag rejects False payload" {
                spec "https://www.rfc-editor.org/rfc/rfc4252.html#section-7"
                given "Public Key Signature Flag Assembles a False Payload" {
                    let blob = mock_public_key_blob();
                    let mut data = Vec::new();
                    append_string(&mut data, b"session-id");
                    data.push(SSH_MSG_USERAUTH_REQUEST);
                    append_string(&mut data, b"user");
                    append_string(&mut data, SERVICE_SSH_CONNECTION);
                    append_string(&mut data, METHOD_PUBLICKEY);
                    data.push(0);
                    append_string(&mut data, SshKeyAlgorithm::SkEcdsaSha2NistP256.as_bytes());
                    append_string(&mut data, &blob.to_bytes());
                }
                when "Analyze Payload" {
                    let result = SshUserauthSigningPayload::from_bytes(&data);
                }
                then "Rejected with an error with the actual value of the signature flag" {
                    assert!(matches!(
                        result,
                        Err(SshSigningPayloadError::PublicKeyBooleanFalse { actual: 0 })
                    ));
                }
            }
            case "Reject payload with extra data at the end" {
                why "In order not to overlook DATA outside the authentication payload, uniquely parsed the entire signature object."
                spec "https://www.rfc-editor.org/rfc/rfc4252.html#section-7"
                given "Add extra bytes to the end of a valid payload" {
                    let mut data = valid_payload();
                    data.push(0x00);
                }
                when "Analyze Payload" {
                    let result = SshUserauthSigningPayload::from_bytes(&data);
                }
                then "Rejected for errors with extra bytes" {
                    assert!(matches!(
                        result,
                        Err(SshSigningPayloadError::TrailingData { trailing_bytes: 1 })
                    ));
                }
            }
            case "Reject a cut off Payload in the middle" {
                why "Because it does not allow signing before all FIELDs required for public key authentication signatures are aligned"
                spec "https://www.rfc-editor.org/rfc/rfc4252.html#section-7"
                given "Prepare a valid payload and all cutting positions that are shorter than full length" {
                    let data = valid_payload();
                    let lengths = 0..data.len();
                }
                when "Analyze the cut PAYLOAD at each position" {
                    let results = lengths
                        .map(|len| SshUserauthSigningPayload::from_bytes(&data[..len]))
                        .collect::<Vec<_>>();
                }
                then "Analysis fails at any cutting position" {
                    assert!(results.into_iter().all(|result| result.is_err()));
                }
            }
            case "Reject Host-Bound Payload without Host Key Blob" {
                spec "https://github.com/openssh/openssh-portable/blob/master/PROTOCOL#L321-L338"
                given "Assemble a Host-Bound Payload that only the Host Key Blob is missing" {
                    let blob = mock_public_key_blob();
                    let mut data = Vec::new();
                    append_string(&mut data, b"session-id");
                    data.push(SSH_MSG_USERAUTH_REQUEST);
                    append_string(&mut data, b"user");
                    append_string(&mut data, SERVICE_SSH_CONNECTION);
                    append_string(&mut data, METHOD_PUBLICKEY_HOSTBOUND);
                    data.push(1);
                    append_string(&mut data, SshKeyAlgorithm::SkEcdsaSha2NistP256.as_bytes());
                    append_string(&mut data, &blob.to_bytes());
                }
                when "Analyze Payload" {
                    let result = SshUserauthSigningPayload::from_bytes(&data);
                }
                then "Rejected for missing host_key_blob" {
                    assert!(matches!(
                        result,
                        Err(SshSigningPayloadError::Truncated {
                            field: "host_key_blob"
                        })
                    ));
                }
            }
            case "Rejects a host-bound userauth payload containing a malformed host key" {
                spec "https://github.com/openssh/openssh-portable/blob/master/PROTOCOL#L321-L338"
                given "a structurally complete host-bound payload whose host key is not an SSH public key" {
                    let blob = mock_public_key_blob();
                    let data = build_hostbound_payload(
                        b"session-id",
                        SshKeyAlgorithm::SkEcdsaSha2NistP256.as_bytes(),
                        &blob.to_bytes(),
                        b"not-an-ssh-public-key",
                    );
                }
                when "the host-bound payload is parsed" {
                    let result = SshUserauthSigningPayload::from_bytes(&data);
                }
                then "the malformed host key is reported instead of being accepted as opaque bytes" {
                    assert!(matches!(
                        result,
                        Err(SshSigningPayloadError::InvalidKeyBlob {
                            field: "host_key_blob",
                            ..
                        })
                    ));
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "SshUserauthSigningPayload::from_bytes" {
            case "Rejects a userauth payload whose algorithm differs from its embedded key blob" {
                spec "https://www.rfc-editor.org/rfc/rfc4252.html#section-7"
                given "a userauth payload declares ssh-ed25519 but embeds an SK-ECDSA public key" {
                    let blob = mock_public_key_blob();
                    let data = build_publickey_payload(b"session-id", b"ssh-ed25519", &blob.to_bytes());
                }
                when "the userauth payload is parsed" {
                    let result = SshUserauthSigningPayload::from_bytes(&data);
                }
                then "the inconsistent algorithm declaration is rejected" {
                    assert!(matches!(
                        result,
                        Err(SshSigningPayloadError::KeyAlgorithmMismatch)
                    ));
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "SshUserauthSigningPayload::matches_key_blob" {
            case "Matches the public key embedded in a userauth payload" {
                why "the signing policy must bind authorization to the exact public key carried by the SSH userauth payload"
                given "a parsed userauth payload and the same SK-ECDSA public key" {
                    let blob = mock_public_key_blob();
                    let data = valid_payload();
                    let parsed = SshUserauthSigningPayload::from_bytes(&data).unwrap();
                }
                when "the payload is compared with the expected public key" {
                    let matches = parsed.matches_key_blob(&blob);
                }
                then "the keys match" {
                    assert!(matches);
                }
            }
            case "Does not match a different public key" {
                why "a valid but different public key must not satisfy authorization for the key embedded in the payload"
                given "a userauth payload containing a different valid SK-ECDSA public key" {
                    let blob = mock_public_key_blob();
                    let different_blob = SshPublicKey::from_sk_ecdsa_pubkey(
                        p256::SecretKey::from_slice(&[2; 32])
                            .unwrap()
                            .public_key()
                            .to_sec1_point(false)
                            .as_bytes(),
                        "ssh:",
                    )
                    .unwrap();
                    let data = build_publickey_payload(
                        b"session-id",
                        SshKeyAlgorithm::SkEcdsaSha2NistP256.as_bytes(),
                        &different_blob.to_bytes(),
                    );
                    let parsed = SshUserauthSigningPayload::from_bytes(&data).unwrap();
                }
                when "the payload is compared with the expected public key" {
                    let matches = parsed.matches_key_blob(&blob);
                }
                then "the keys do not match" {
                    assert!(!matches);
                }
            }
        }
    }
}
