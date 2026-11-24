use crate::{
    EcdsaP256SshSignature, SkEcdsaP256SshSignature, SshAgentCodecError, SshAgentIdentity,
    SshAgentRequest, SshAgentResponse, SshAgentSignFlags, SshPublicKeyBlob, SshSignature,
};

use crate::test_support::{append_string, unsupported_public_key};

fn supported_agent_reply_message_types() -> [u8; 5] {
    let signature = SshSignature::Ecdsa(EcdsaP256SshSignature::new([1], [2]));
    [
        SshAgentResponse::Failure.to_bytes().unwrap()[0],
        SshAgentResponse::ExtensionFailure.to_bytes().unwrap()[0],
        SshAgentResponse::Success.to_bytes().unwrap()[0],
        SshAgentResponse::IdentitiesAnswer { identities: vec![] }
            .to_bytes()
            .unwrap()[0],
        SshAgentResponse::Signature { signature }
            .to_bytes()
            .unwrap()[0],
    ]
}

keyringd_testkit::scenario! {
    group "RFC 9987 request wire messages" {
        case "Decodes the fixed SSH_AGENTC_REQUEST_IDENTITIES message" {
            spec "https://www.rfc-editor.org/rfc/rfc9987.html#section-5.5"
            spec "https://www.rfc-editor.org/rfc/rfc9987.html#section-8.1"

            verify "message type 11 with no contents decodes as an identities request" {
                assert!(matches!(
                    SshAgentRequest::from_bytes(&[11]),
                    Ok(SshAgentRequest::Identities)
                ));
            }
        }

        case "Rejects contents appended to SSH_AGENTC_REQUEST_IDENTITIES" {
            spec "https://www.rfc-editor.org/rfc/rfc9987.html#section-5.5"

            verify "the contents-free request reports every appended byte as trailing data" {
                assert!(matches!(
                    SshAgentRequest::from_bytes(&[11, 0xaa, 0xbb]),
                    Err(SshAgentCodecError::TrailingData { trailing_bytes: 2 })
                ));
            }
        }

        case "Decodes an SSH_AGENTC_SIGN_REQUEST fixed vector" {
            spec "https://www.rfc-editor.org/rfc/rfc9987.html#section-5.6"
            spec "https://www.rfc-editor.org/rfc/rfc9987.html#section-8.3"

            given "a literal sign request containing an opaque public key, binary data, and both defined flags" {
                let key = unsupported_public_key(b"future-key@example.com", &[0x10, 0x20]);
                let mut payload = vec![13];
                append_string(&mut payload, &key);
                append_string(&mut payload, &[0x00, 0xff, 0x7f]);
                payload.extend_from_slice(&6_u32.to_be_bytes());
            }
            when "the public request decoder parses the vector" {
                let request = SshAgentRequest::from_bytes(&payload).unwrap();
            }
            then "the key, opaque data, and complete flag bitfield are preserved" {
                let SshAgentRequest::Sign(request) = request else {
                    panic!("expected a sign request");
                };
                assert_eq!(request.key_blob().to_bytes(), key);
                assert_eq!(request.data(), [0x00, 0xff, 0x7f]);
                assert_eq!(request.flags().bits(), 6);
                assert!(request.flags().contains(SshAgentSignFlags::RSA_SHA2_256));
                assert!(request.flags().contains(SshAgentSignFlags::RSA_SHA2_512));
                assert_eq!(request.flags().unknown_bits(), 0);
            }
        }

        case "Rejects every truncation of an SSH_AGENTC_SIGN_REQUEST" {
            spec "https://www.rfc-editor.org/rfc/rfc9987.html#section-5.6"

            verify "no proper prefix of a complete sign request is accepted" {
                let key = unsupported_public_key(b"future-key@example.com", &[1, 2, 3]);
                let mut payload = vec![13];
                append_string(&mut payload, &key);
                append_string(&mut payload, b"payload");
                payload.extend_from_slice(&0_u32.to_be_bytes());

                for length in 0..payload.len() {
                    assert!(
                        SshAgentRequest::from_bytes(&payload[..length]).is_err(),
                        "accepted a sign request truncated to {length} bytes"
                    );
                }
            }
        }

        case "Decodes an SSH_AGENTC_EXTENSION request with opaque binary contents" {
            spec "https://www.rfc-editor.org/rfc/rfc9987.html#section-5.8"

            given "an extension request with a UTF-8 extension name and non-UTF-8 contents" {
                let mut payload = vec![27];
                append_string(&mut payload, b"example@example.com");
                payload.extend_from_slice(&[0x00, 0xff, 0x80]);
            }
            when "the request is decoded" {
                let request = SshAgentRequest::from_bytes(&payload).unwrap();
            }
            then "only the name is interpreted as UTF-8 and the remaining contents stay opaque" {
                let SshAgentRequest::Extension(request) = request else {
                    panic!("expected an extension request");
                };
                assert_eq!(request.name(), "example@example.com");
                assert_eq!(request.data(), [0x00, 0xff, 0x80]);
            }
        }

        case "Decodes an SSH_AGENTC_EXTENSION request with empty contents" {
            spec "https://www.rfc-editor.org/rfc/rfc9987.html#section-5.8"

            verify "the extension name is complete even when no request-specific bytes follow it" {
                let mut payload = vec![27];
                append_string(&mut payload, b"query");
                let request = SshAgentRequest::from_bytes(&payload).unwrap();
                let SshAgentRequest::Extension(request) = request else {
                    panic!("expected an extension request");
                };
                assert_eq!(request.name(), "query");
                assert!(request.data().is_empty());
            }
        }

        case "Does not reuse legacy SSH protocol message numbers" {
            spec "https://www.rfc-editor.org/rfc/rfc9987.html#section-8.1.1"

            verify "every SSH version 1 reservation remains unknown to the generic SSH Agent request decoder" {
                let reserved = [1_u8, 2, 3, 4, 7, 8, 9, 10, 15, 16, 24];
                for message_type in reserved {
                    assert!(matches!(
                        SshAgentRequest::from_bytes(&[message_type]),
                        Ok(SshAgentRequest::Unknown {
                            message_type: decoded,
                            data,
                        }) if decoded == message_type && data.is_empty()
                    ));
                }
                assert!(supported_agent_reply_message_types()
                    .iter()
                    .all(|message_type| !reserved.contains(message_type)));
            }
        }

        case "Does not use reserved message number zero" {
            spec "https://www.rfc-editor.org/rfc/rfc9987.html#section-8.1.1"

            verify "zero remains unknown to the generic SSH Agent request decoder" {
                assert!(matches!(
                    SshAgentRequest::from_bytes(&[0]),
                    Ok(SshAgentRequest::Unknown {
                        message_type: 0,
                        data,
                    }) if data.is_empty()
                ));
                assert!(!supported_agent_reply_message_types().contains(&0));
            }
        }

        case "Does not use Private Use message numbers in the generic implementation" {
            spec "https://www.rfc-editor.org/rfc/rfc9987.html#section-8.1.1"

            verify "every value from 240 through 255 remains unknown to the generic SSH Agent request decoder" {
                for message_type in 240_u8..=255 {
                    assert!(matches!(
                        SshAgentRequest::from_bytes(&[message_type]),
                        Ok(SshAgentRequest::Unknown {
                            message_type: decoded,
                            data,
                        }) if decoded == message_type && data.is_empty()
                    ));
                }
                assert!(supported_agent_reply_message_types()
                    .iter()
                    .all(|message_type| !(240..=255).contains(message_type)));
            }
        }

        case "Preserves every registered but unsupported client request number" {
            spec "https://www.rfc-editor.org/rfc/rfc9987.html#section-8.1"

            verify "registered requests outside the implemented subset remain distinct unknown requests" {
                for message_type in [17_u8, 18, 19, 20, 21, 22, 23, 25, 26] {
                    assert!(matches!(
                        SshAgentRequest::from_bytes(&[message_type]),
                        Ok(SshAgentRequest::Unknown {
                            message_type: decoded,
                            data,
                        }) if decoded == message_type && data.is_empty()
                    ));
                }
            }
        }

    }
}

keyringd_testkit::scenario! {
    group "RFC 9987 response wire messages" {
        case "Encodes generic success and failure as single-byte responses" {
            spec "https://www.rfc-editor.org/rfc/rfc9987.html#section-5.1"
            spec "https://www.rfc-editor.org/rfc/rfc9987.html#section-8.1"

            verify "the generic response payloads contain only their registered message type" {
                assert_eq!(SshAgentResponse::Failure.to_bytes().unwrap(), [5]);
                assert_eq!(SshAgentResponse::Success.to_bytes().unwrap(), [6]);
            }
        }

        case "Encodes an empty SSH_AGENT_IDENTITIES_ANSWER fixed vector" {
            spec "https://www.rfc-editor.org/rfc/rfc9987.html#section-5.5"

            verify "the response contains type 12 and a zero key count" {
                assert_eq!(
                    SshAgentResponse::IdentitiesAnswer {
                        identities: Vec::new(),
                    }
                    .to_bytes()
                    .unwrap(),
                    [12, 0, 0, 0, 0]
                );
            }
        }

        case "Encodes identity key blobs and UTF-8 comments in list order" {
            spec "https://www.rfc-editor.org/rfc/rfc9987.html#section-5.5"

            given "two opaque public keys with an empty comment and a multibyte UTF-8 comment" {
                let first_key_bytes = unsupported_public_key(b"first@example.com", &[1]);
                let second_key_bytes = unsupported_public_key(b"second@example.com", &[2, 3]);
                let response = SshAgentResponse::IdentitiesAnswer {
                    identities: vec![
                        SshAgentIdentity::new(
                            SshPublicKeyBlob::from_bytes(&first_key_bytes).unwrap(),
                            "",
                        ),
                        SshAgentIdentity::new(
                            SshPublicKeyBlob::from_bytes(&second_key_bytes).unwrap(),
                            "clé-two",
                        ),
                    ],
                };
                let mut expected = vec![12, 0, 0, 0, 2];
                append_string(&mut expected, &first_key_bytes);
                append_string(&mut expected, b"");
                append_string(&mut expected, &second_key_bytes);
                append_string(&mut expected, "clé-two".as_bytes());
            }
            when "the response is encoded" {
                let encoded = response.to_bytes().unwrap();
            }
            then "the fixed vector contains the declared count followed by both identity records" {
                assert_eq!(encoded, expected);
            }
        }

        case "Encodes an SSH_AGENT_SIGN_RESPONSE fixed vector" {
            spec "https://www.rfc-editor.org/rfc/rfc9987.html#section-5.6"

            given "a signature using a syntactically valid future algorithm" {
                let signature = SshSignature::Unsupported { algorithm: "signature@example.com".to_owned(), data: vec![0xaa, 0xbb] };
                let response = SshAgentResponse::Signature { signature };

                let mut signature_blob = Vec::new();
                append_string(&mut signature_blob, b"signature@example.com");
                append_string(&mut signature_blob, &[0xaa, 0xbb]);
                let mut expected = vec![14];
                append_string(&mut expected, &signature_blob);
            }
            when "the response is encoded" {
                let encoded = response.to_bytes().unwrap();
            }
            then "the payload contains type 14 followed by one SSH string carrying the signature blob" {
                assert_eq!(encoded, expected);
            }
        }

        case "Encodes every supported Agent reply with its registered message number" {
            spec "https://www.rfc-editor.org/rfc/rfc9987.html#section-8.1"

            verify "failure, success, identities, and signature replies use 5, 6, 12, and 14" {
                let signature = SshSignature::Ecdsa(EcdsaP256SshSignature::new([1], [2]));
                let replies = [
                    SshAgentResponse::Failure.to_bytes().unwrap(),
                    SshAgentResponse::Success.to_bytes().unwrap(),
                    SshAgentResponse::IdentitiesAnswer { identities: vec![] }
                        .to_bytes()
                        .unwrap(),
                    SshAgentResponse::Signature { signature }
                        .to_bytes()
                        .unwrap(),
                ];
                assert_eq!(replies.map(|reply| reply[0]), [5, 6, 12, 14]);
                assert_eq!(SshAgentResponse::ExtensionFailure.to_bytes().unwrap(), [28]);
            }
        }

        case "Encodes an ECDSA signature in the algorithm-specific SSH format" {
            spec "https://www.rfc-editor.org/rfc/rfc9987.html#section-5.6"

            given "positive ECDSA r and s integers whose high bits require no sign padding" {
                let signature = EcdsaP256SshSignature::new([1], [2]).into_signature();
                let mut expected_data = Vec::new();
                append_string(&mut expected_data, &[1]);
                append_string(&mut expected_data, &[2]);
                let mut expected = Vec::new();
                append_string(&mut expected, b"ecdsa-sha2-nistp256");
                append_string(&mut expected, &expected_data);
            }
            when "the RFC 5656 ECDSA signature is encoded as an SSH signature blob" {
                let encoded = signature.to_bytes();
            }
            then "the algorithm name is followed by a string containing the r and s mpints" {
                assert_eq!(encoded, expected);
            }
        }

        case "Encodes an OpenSSH security-key signature without nesting its trailing fields" {
            spec "https://github.com/openssh/openssh-portable/blob/master/PROTOCOL.u2f#ssh-u2f-key-formats"

            given "an SK-ECDSA signature with fixed r, s, flags, and counter" {
                let signature = SkEcdsaP256SshSignature::new(
                    EcdsaP256SshSignature::new([1], [2]),
                    1,
                    42,
                )
                .into_signature();
                let mut ecdsa = Vec::new();
                append_string(&mut ecdsa, &[1]);
                append_string(&mut ecdsa, &[2]);
                let mut expected = Vec::new();
                append_string(&mut expected, b"sk-ecdsa-sha2-nistp256@openssh.com");
                append_string(&mut expected, &ecdsa);
                expected.push(1);
                expected.extend_from_slice(&42_u32.to_be_bytes());
            }
            when "the signature is encoded and decoded" {
                let encoded = signature.to_bytes();
                let decoded = SshSignature::from_bytes(&encoded).unwrap();
            }
            then "flags and counter follow the ECDSA signature string as top-level fields" {
                assert_eq!(encoded, expected);
                assert_eq!(decoded, signature);
            }
        }
    }
}
