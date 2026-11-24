use super::SshSigningPayloadError;

const SSHSIG_MAGIC: &[u8] = b"SSHSIG";

/// The signed-data format from OpenSSH `PROTOCOL.sshsig`.
#[derive(Debug)]
pub struct SshsigSigningPayload<'a> {
    namespace: &'a [u8],
    reserved: &'a [u8],
    hash_algorithm: &'a [u8],
    message_hash: &'a [u8],
}

impl<'a> SshsigSigningPayload<'a> {
    pub fn from_bytes(data: &'a [u8]) -> Result<Self, SshSigningPayloadError> {
        let rest = data
            .strip_prefix(SSHSIG_MAGIC)
            .ok_or(SshSigningPayloadError::MalformedSshsig("missing magic"))?;
        let (namespace, rest) = read_string(rest).ok_or(
            SshSigningPayloadError::MalformedSshsig("truncated namespace"),
        )?;
        if namespace.is_empty() {
            return Err(SshSigningPayloadError::MalformedSshsig(
                "namespace is empty",
            ));
        }
        let (reserved, rest) = read_string(rest).ok_or(SshSigningPayloadError::MalformedSshsig(
            "truncated reserved field",
        ))?;
        let (hash_algorithm, rest) = read_string(rest).ok_or(
            SshSigningPayloadError::MalformedSshsig("truncated hash algorithm"),
        )?;
        let (message_hash, rest) = read_string(rest).ok_or(
            SshSigningPayloadError::MalformedSshsig("truncated message digest"),
        )?;
        let expected_hash_length = match hash_algorithm {
            b"sha256" => 32,
            b"sha512" => 64,
            _ => {
                return Err(SshSigningPayloadError::MalformedSshsig(
                    "unsupported hash algorithm",
                ));
            }
        };
        if message_hash.len() != expected_hash_length {
            return Err(SshSigningPayloadError::MalformedSshsig(
                "message digest has the wrong length",
            ));
        }
        if !rest.is_empty() {
            return Err(SshSigningPayloadError::MalformedSshsig("trailing data"));
        }

        Ok(Self {
            namespace,
            reserved,
            hash_algorithm,
            message_hash,
        })
    }

    pub fn namespace(&self) -> &[u8] {
        self.namespace
    }

    pub fn reserved(&self) -> &[u8] {
        self.reserved
    }

    pub fn hash_algorithm(&self) -> &[u8] {
        self.hash_algorithm
    }

    pub fn message_hash(&self) -> &[u8] {
        self.message_hash
    }
}

fn read_string(bytes: &[u8]) -> Option<(&[u8], &[u8])> {
    let length = u32::from_be_bytes(bytes.get(..4)?.try_into().ok()?) as usize;
    let value = bytes.get(4..4 + length)?;
    Some((value, &bytes[4 + length..]))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sshsig(namespace: &[u8], reserved: &[u8], algorithm: &[u8], digest: &[u8]) -> Vec<u8> {
        let mut data = SSHSIG_MAGIC.to_vec();
        for value in [namespace, reserved, algorithm, digest] {
            data.extend((value.len() as u32).to_be_bytes());
            data.extend(value);
        }
        data
    }

    keyringd_testkit::scenario! {
        group "SshsigSigningPayload::from_bytes" {
            case "Parses a SHA-256 SSHSIG signed-data payload" {
                spec "https://github.com/openssh/openssh-portable/blob/master/PROTOCOL.sshsig#L75-L89"
                spec "https://github.com/openssh/openssh-portable/blob/master/PROTOCOL.sshsig#L57-L61"
                given "a complete SSHSIG payload containing a SHA-256 message hash" {
                    let encoded = sshsig(b"git", b"", b"sha256", &[7; 32]);
                }
                when "the signed-data payload is parsed" {
                    let parsed = SshsigSigningPayload::from_bytes(&encoded).unwrap();
                }
                then "all protocol fields are exposed without modification" {
                    assert_eq!(parsed.namespace(), b"git");
                    assert_eq!(parsed.reserved(), b"");
                    assert_eq!(parsed.hash_algorithm(), b"sha256");
                    assert_eq!(parsed.message_hash(), &[7; 32]);
                }
            }
            case "Parses a SHA-512 SSHSIG signed-data payload" {
                spec "https://github.com/openssh/openssh-portable/blob/master/PROTOCOL.sshsig#L75-L89"
                spec "https://github.com/openssh/openssh-portable/blob/master/PROTOCOL.sshsig#L57-L61"
                verify "a 64-byte SHA-512 message hash is accepted" {
                    let encoded = sshsig(b"git", b"", b"sha512", &[7; 64]);
                    assert!(SshsigSigningPayload::from_bytes(&encoded).is_ok());
                }
            }
            case "Accepts an opaque non-UTF-8 namespace" {
                spec "https://www.rfc-editor.org/rfc/rfc4251.html#section-5"
                spec "https://github.com/openssh/openssh-portable/blob/master/PROTOCOL.sshsig#L48-L55"
                verify "the SSH string namespace is not incorrectly restricted to UTF-8" {
                    let encoded = sshsig(&[0xff], b"", b"sha256", &[0; 32]);
                    assert!(SshsigSigningPayload::from_bytes(&encoded).is_ok());
                }
            }
            case "Accepts a namespace containing a control byte" {
                spec "https://www.rfc-editor.org/rfc/rfc4251.html#section-5"
                spec "https://github.com/openssh/openssh-portable/blob/master/PROTOCOL.sshsig#L48-L55"
                verify "the parser applies only the non-empty namespace requirement from PROTOCOL.sshsig" {
                    let encoded = sshsig(b"git\n", b"", b"sha256", &[0; 32]);
                    assert!(SshsigSigningPayload::from_bytes(&encoded).is_ok());
                }
            }
            case "Accepts a non-empty reserved field" {
                spec "https://github.com/openssh/openssh-portable/blob/master/PROTOCOL.sshsig#L53-L55"
                verify "future extension data in the reserved field is preserved and ignored" {
                    let encoded = sshsig(b"git", b"reserved", b"sha256", &[0; 32]);
                    let parsed = SshsigSigningPayload::from_bytes(&encoded).unwrap();
                    assert_eq!(parsed.reserved(), b"reserved");
                }
            }
            case "Rejects an empty namespace" {
                spec "https://github.com/openssh/openssh-portable/blob/master/PROTOCOL.sshsig#L48-L55"
                verify "the mandatory SSHSIG namespace cannot be empty" {
                    let encoded = sshsig(b"", b"", b"sha256", &[0; 32]);
                    assert!(matches!(
                        SshsigSigningPayload::from_bytes(&encoded),
                        Err(SshSigningPayloadError::MalformedSshsig(
                            "namespace is empty"
                        ))
                    ));
                }
            }
            case "Rejects an unsupported hash algorithm" {
                spec "https://github.com/openssh/openssh-portable/blob/master/PROTOCOL.sshsig#L57-L61"
                verify "SHA-1 is not one of the hash algorithms supported by PROTOCOL.sshsig" {
                    let encoded = sshsig(b"git", b"", b"sha1", &[0; 20]);
                    assert!(matches!(
                        SshsigSigningPayload::from_bytes(&encoded),
                        Err(SshSigningPayloadError::MalformedSshsig(
                            "unsupported hash algorithm"
                        ))
                    ));
                }
            }
            case "Rejects a message hash with the wrong length" {
                spec "https://github.com/openssh/openssh-portable/blob/master/PROTOCOL.sshsig#L57-L61"
                verify "a 31-byte SHA-256 message hash is rejected" {
                    let encoded = sshsig(b"git", b"", b"sha256", &[0; 31]);
                    assert!(matches!(
                        SshsigSigningPayload::from_bytes(&encoded),
                        Err(SshSigningPayloadError::MalformedSshsig(
                            "message digest has the wrong length"
                        ))
                    ));
                }
            }
            case "Rejects trailing data" {
                why "the parser must consume exactly one SSHSIG signed-data value and reject ambiguous suffix bytes"
                spec "https://github.com/openssh/openssh-portable/blob/master/PROTOCOL.sshsig#L75-L89"
                verify "one byte after the final SSH string is reported as trailing data" {
                    let mut encoded = sshsig(b"git", b"", b"sha256", &[0; 32]);
                    encoded.push(0);
                    assert!(matches!(
                        SshsigSigningPayload::from_bytes(&encoded),
                        Err(SshSigningPayloadError::MalformedSshsig("trailing data"))
                    ));
                }
            }
            case "Rejects every truncation of a complete payload" {
                why "every field in the SSHSIG signed-data structure is required before signing is authorized"
                spec "https://github.com/openssh/openssh-portable/blob/master/PROTOCOL.sshsig#L75-L89"
                verify "every prefix after the magic and before the final byte is malformed" {
                    let encoded = sshsig(b"git", b"", b"sha256", &[0; 32]);
                    for length in SSHSIG_MAGIC.len()..encoded.len() {
                        assert!(SshsigSigningPayload::from_bytes(&encoded[..length]).is_err());
                    }
                }
            }
        }
    }
}
