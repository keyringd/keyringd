use crate::webauthn::AttestationObject;
use crate::webauthn::AttestationStatement;
use crate::webauthn::AuthenticatorData;
use crate::webauthn::ProtocolError;

keyringd_testkit::scenario! {
    group "AttestationObject::from_cbor_bytes" {
            case "None Attestation can be transformed back and forth" {
                spec "https://www.w3.org/TR/webauthn-3/#sctn-generating-an-attestation-object"
                spec "https://www.w3.org/TR/webauthn-3/#sctn-none-attestation"

                given "Prepare RAW" {
                    let mut raw = vec![0x55; 32];
                    raw.extend_from_slice(&[0x01, 0, 0, 0, 0]);
                }
                when "Call AuthenticatorData::from_bytes" {
                    let value = AttestationObject {
                        authenticator_data: AuthenticatorData::from_bytes(&raw).unwrap(),
                        attestation_statement: AttestationStatement::None,
                    };
                    let encoded = value.to_cbor_bytes().unwrap();
                }
                then "The none attestation after decode matches before encode" {
                    assert_eq!(AttestationObject::from_cbor_bytes(&encoded).unwrap(), value);
                }

            }
            case "Reject missing or duplicate fields" {
                spec "https://www.w3.org/TR/webauthn-3/#sctn-generating-an-attestation-object"

                given "Prepare a CBOR with a missing FMT and a Duplicate CBOR" {
                    let missing_fmt = [0xa0];
                    let duplicate_fmt = hex_literal::hex!(
                        "a2
                         63 666d74 64 6e6f6e65
                         63 666d74 64 6e6f6e65"
                    );
                }
                when "Decode both as Attestation Object" {
                    let missing_result = AttestationObject::from_cbor_bytes(&missing_fmt);
                    let duplicate_result = AttestationObject::from_cbor_bytes(&duplicate_fmt);
                }
                then "Required Field Missing and Duplicated FIELD Rejected" {
                    assert!(matches!(missing_result, Err(ProtocolError::MissingField("fmt"))));
                    assert!(duplicate_result.is_err());
                }

            }
    }
}
