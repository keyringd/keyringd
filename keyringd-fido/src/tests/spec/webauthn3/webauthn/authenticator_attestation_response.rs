use crate::{ProtocolError, webauthn::*};
use base64::Engine;
use p256::elliptic_curve::sec1::ToSec1Point;
use p256::pkcs8::DecodePublicKey;

fn registration_authenticator_data() -> AuthenticatorData {
    let mut bytes = vec![0x22; 32];
    bytes.push(0x01 | 0x40);
    bytes.extend_from_slice(&0u32.to_be_bytes());
    bytes.extend_from_slice(&[0x33; 16]);
    bytes.extend_from_slice(&3u16.to_be_bytes());
    bytes.extend_from_slice(&[1, 2, 3]);
    bytes.extend_from_slice(&[0xa5, 0x01, 0x02, 0x03, 0x26, 0x20, 0x01, 0x21, 0x58, 0x20]);
    bytes.extend_from_slice(&hex_literal::hex!(
        "6b17d1f2e12c4247f8bce6e563a440f277037d812deb33a0f4a13945d898c296"
    ));
    bytes.extend_from_slice(&[0x22, 0x58, 0x20]);
    bytes.extend_from_slice(&hex_literal::hex!(
        "4fe342e2fe1a7f9b8ee7eb4a7c0f9e162bce33576b315ececbb6406837bf51f5"
    ));
    AuthenticatorData::from_bytes(&bytes).unwrap()
}

keyringd_testkit::scenario! {
    group "AuthenticatorAttestationResponse JSON representation" {
            case "Round-trips a registration response with all derived members" {
                spec "https://www.w3.org/TR/webauthn-3/#iface-authenticatorattestationresponse"
                spec "https://www.w3.org/TR/webauthn-3/#dictdef-authenticatorattestationresponsejson"
                spec "https://www.w3.org/TR/webauthn-3/#dom-authenticatorattestationresponse-getauthenticatordata"
                spec "https://www.w3.org/TR/webauthn-3/#dom-authenticatorattestationresponse-getpublickeyalgorithm"
                spec "https://www.w3.org/TR/webauthn-3/#dom-authenticatorattestationresponse-getpublickey"
                spec "https://www.w3.org/TR/webauthn-3/#sctn-public-key-easy"

                given "a none-attestation response containing an ES256 credential and every standardized transport" {
                    let authenticator_data = registration_authenticator_data();
                    let attestation_object = AttestationObject::new(authenticator_data.clone(),
                        AttestationStatement::None,
                    );
                    let transports = vec![
                        AuthenticatorTransport::ble(),
                        AuthenticatorTransport::hybrid(),
                        AuthenticatorTransport::internal(),
                        AuthenticatorTransport::nfc(),
                        AuthenticatorTransport::smart_card(),
                        AuthenticatorTransport::usb(),
                    ];
                    let response = AuthenticatorAttestationResponse::from_attestation_object(
                        b"client-data".to_vec(),
                        attestation_object,
                        transports,
                    )
                    .unwrap();
                }
                when "the response is serialized to JSON and parsed again" {
                    let json = response.to_json_bytes().unwrap();
                    let decoded = AuthenticatorAttestationResponse::from_json_bytes(&json).unwrap();
                }
                then "the attestation object, transport hints, public key, and algorithm remain consistent" {
                    assert_eq!(decoded, response);
                    assert_eq!(decoded.authenticator_data().unwrap(), authenticator_data);
                    assert_eq!(decoded.public_key_algorithm().unwrap(), CoseAlgorithmIdentifier::ES256);
                    let der = decoded.public_key().unwrap().unwrap();
                    let public_key = p256::PublicKey::from_public_key_der(&der).unwrap();
                    let expected_point = authenticator_data.attested_credential_data().unwrap()
                        .credential_public_key().es256_sec1_public_key().unwrap();
                    assert_eq!(public_key.to_sec1_point(false).as_bytes(), expected_point);
                    let json_value: serde_json::Value = serde_json::from_slice(&json).unwrap();
                    assert_eq!(
                        base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(json_value["publicKey"].as_str().unwrap()).unwrap(),
                        der
                    );
                    assert_eq!(
                        decoded
                            .transports()
                            .iter()
                            .map(AuthenticatorTransport::as_str)
                            .collect::<Vec<_>>(),
                        ["ble", "hybrid", "internal", "nfc", "smart-card", "usb"]
                    );
                }
            }
            case "Rejects transport hints that are not unique and lexicographically ordered" {
                spec "https://www.w3.org/TR/webauthn-3/#dom-authenticatorattestationresponse-transports-slot"

                given "an otherwise valid attestation response with duplicate USB transport hints" {
                    let attestation_object = AttestationObject::new(registration_authenticator_data(),
                        AttestationStatement::None,
                    );
                    let transports = vec![
                        AuthenticatorTransport::usb(),
                        AuthenticatorTransport::usb(),
                    ];
                }
                when "the response is constructed from the attestation object and transport hints" {
                    let result = AuthenticatorAttestationResponse::from_attestation_object(
                        b"client-data".to_vec(),
                        attestation_object,
                        transports,
                    );
                }
                then "the invalid transport sequence is rejected" {
                    assert!(matches!(
                        result,
                        Err(ProtocolError::UnexpectedType {
                            field: "AuthenticatorAttestationResponse.transports",
                            ..
                        })
                    ));
                }
            }
            case "Rejects a derived member that disagrees with the attestation object" {
                spec "https://www.w3.org/TR/webauthn-3/#dictdef-authenticatorattestationresponsejson"

                given "a valid registration response JSON whose publicKeyAlgorithm is replaced" {
                    let attestation_object = AttestationObject::new(registration_authenticator_data(),
                        AttestationStatement::None,
                    );
                    let response = AuthenticatorAttestationResponse::from_attestation_object(
                        b"client-data".to_vec(),
                        attestation_object,
                        Vec::new(),
                    )
                    .unwrap();
                    let mut json: serde_json::Value =
                        serde_json::from_slice(&response.to_json_bytes().unwrap()).unwrap();
                    json["publicKeyAlgorithm"] = serde_json::json!(-257);
                }
                when "the inconsistent JSON representation is parsed" {
                    let result = AuthenticatorAttestationResponse::from_json_bytes(
                        &serde_json::to_vec(&json).unwrap(),
                    );
                }
                then "the response is rejected instead of trusting the inconsistent derived member" {
                    assert!(matches!(
                        result,
                        Err(ProtocolError::UnexpectedType {
                            field: "AuthenticatorAttestationResponseJSON derived members",
                            ..
                        })
                    ));
                }
            }
    }
}
