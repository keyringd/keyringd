use crate::codec::CborCodec;
use crate::codec::CborItem as Value;
use crate::webauthn::CredentialPublicKey;
use crate::webauthn::*;

keyringd_testkit::scenario! {
    group "CredentialPublicKey::from_cbor_bytes" {
            case "EC2 key requires algorithm, curve and coordinates" {
                spec "https://www.rfc-editor.org/rfc/rfc9052.html#section-7"
                spec "https://www.rfc-editor.org/rfc/rfc9053.html#section-2.1"

                verify "Reject Missing and Incorrect Algorithm Types/Scope of Each Required Parameter" {
                    let complete = || vec![
                        (Value::integer(1), Value::integer(2)),
                        (Value::integer(3), Value::integer(-7)),
                        (Value::integer(-1), Value::integer(1)),
                        (Value::integer(-2), Value::ByteString(vec![0x11; 32])),
                        (Value::integer(-3), Value::ByteString(vec![0x22; 32])),
                    ];
                    for missing in [3, -1, -2, -3] {
                        let fields = complete()
                            .into_iter()
                            .filter(|(label, _)| *label != Value::integer(missing))
                            .collect();
                        let encoded = CborCodec::encode(Value::Map(fields)).unwrap();
                        assert!(CredentialPublicKey::from_cbor_bytes(&encoded).is_err());
                    }

                    for invalid_algorithm in [
                        Value::TextString("ES256".into()),
                        Value::integer(i64::from(i32::MAX) + 1),
                    ] {
                        let fields = complete()
                            .into_iter()
                            .map(|(label, value)| {
                                if label == Value::integer(3) {
                                    (label, invalid_algorithm.clone())
                                } else {
                                    (label, value)
                                }
                            })
                            .collect();
                        let encoded = CborCodec::encode(Value::Map(fields)).unwrap();
                        assert!(CredentialPublicKey::from_cbor_bytes(&encoded).is_err());
                    }
                }

            }
    }
}

fn es256_key() -> CredentialPublicKey {
    CredentialPublicKey::Ec2(Ec2CredentialPublicKey::new(
        CoseAlgorithmIdentifier::ES256,
        CoseEllipticCurve::P256,
        vec![0x11; 32],
        vec![0x22; 32],
    ))
}

keyringd_testkit::scenario! {
    group "CredentialPublicKey::from_cbor_bytes" {
            case "ES256 key can be converted back and forth and can be output to SEC1 format" {
                spec "https://www.w3.org/TR/webauthn-3/#sctn-encoded-credPubKey-examples"
                spec "https://www.rfc-editor.org/rfc/rfc9052.html#section-7"

                given "Prepare an ES256 Credential Public Key with fixed coordinates" {
                    let key = es256_key();
                }
                when "Perform CBOR round trip and conversion to SEC1 public key" {
                    let decoded = CredentialPublicKey::from_cbor_bytes(&key.to_cbor_bytes().unwrap()).unwrap();
                    let sec1 = key.es256_sec1_public_key().unwrap();
                }
                then "The key is restored and the coordinates are stored in the SEC1 uncompressed format" {
                    assert_eq!(
                        decoded,
                        key
                    );
                    assert_eq!(sec1[0], 4);
                    assert_eq!(&sec1[1..33], &[0x11; 32]);
                    assert_eq!(&sec1[33..], &[0x22; 32]);
                }

            }
            case "Rejects a credential public key that is not canonically encoded" {
                spec "https://www.w3.org/TR/webauthn-3/#sctn-attested-credential-data"
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#message-encoding"

                given "an otherwise valid unknown COSE key whose key-type label uses a non-minimal integer encoding" {
                    let encoded = hex_literal::hex!("a2 18 01 19 03e7 03 3903e6");
                }
                when "the credential public key is decoded" {
                    let result = CredentialPublicKey::from_cbor_bytes(&encoded);
                }
                then "the non-canonical COSE key is rejected before its fields are exposed" {
                    assert!(result.is_err());
                }
            }

    }
}
