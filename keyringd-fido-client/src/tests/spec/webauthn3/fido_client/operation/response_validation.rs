use crate::client::operation::ctap::ResponseValidationError;
use crate::client::operation::ctap::reject_forbidden_ec2_key_parameters;
use crate::client::operation::ctap::validate_attestation_format_identifier;
use crate::client::operation::ctap::validate_authenticator_extension_outputs;
use crate::tests::support::client_operation_authenticator_broker::assert_protocol_error;
use keyringd_fido::CredentialPublicKey;

keyringd_testkit::scenario! {
    case "Rejects optional COSE Key parameters during client response validation" {
        spec "https://www.w3.org/TR/webauthn-3/#sctn-attested-credential-data"

        given "decoded ES256 credential public keys containing each optional common COSE Key parameter" {
            use keyringd_fido::{CoseEllipticCurve, CoseKeyOperation, Ec2CredentialPublicKey};
            use keyringd_fido::webauthn::CoseAlgorithmIdentifier;

            let keys = [
                Ec2CredentialPublicKey::new(
                    CoseAlgorithmIdentifier::ES256,
                    CoseEllipticCurve::P256,
                    vec![0x11; 32],
                    vec![0x22; 32],
                )
                .with_key_id(vec![1]),
                Ec2CredentialPublicKey::new(
                    CoseAlgorithmIdentifier::ES256,
                    CoseEllipticCurve::P256,
                    vec![0x11; 32],
                    vec![0x22; 32],
                )
                .with_key_operations(vec![CoseKeyOperation::new(2)]),
                Ec2CredentialPublicKey::new(
                    CoseAlgorithmIdentifier::ES256,
                    CoseEllipticCurve::P256,
                    vec![0x11; 32],
                    vec![0x22; 32],
                )
                .with_base_initialization_vector(vec![3]),
            ]
            .map(CredentialPublicKey::Ec2);
        }
        when "the client validates each credential public key" {
            let results = keys.each_ref().map(reject_forbidden_ec2_key_parameters);
        }
        then "the decoded values are rejected at the WebAuthn client boundary" {
            for (result, expected_label) in results.into_iter().zip([2, 4, 5]) {
                let error = result.unwrap_err();
                assert_protocol_error::<ResponseValidationError>(&error, |error| {
                    matches!(
                        error,
                        ResponseValidationError::ForbiddenCredentialPublicKeyParameter(label)
                            if *label == expected_label
                    )
                });
            }
        }
    }

    case "Rejects invalid attestation format identifiers during client response validation" {
        spec "https://www.w3.org/TR/webauthn-3/#sctn-attstn-fmt-ids"

        given "attestation statements decoded with overlong, non-ASCII, quote, and backslash format identifiers" {
            use keyringd_fido::webauthn::AttestationStatement;

            let invalid_identifiers = [
                "a".repeat(33),
                "café".to_owned(),
                "bad format".to_owned(),
                "bad\u{1f}format".to_owned(),
                "bad\u{7f}format".to_owned(),
                "bad\"fmt".to_owned(),
                "bad\\fmt".to_owned(),
            ];
            let decoded = invalid_identifiers
                .iter()
                .map(|identifier| {
                    AttestationStatement::from_cbor_bytes(identifier, &[0xa0]).unwrap()
                })
                .collect::<Vec<_>>();
            let maximum = AttestationStatement::from_cbor_bytes(&"a".repeat(32), &[0xa0])
                .unwrap();
        }
        when "the client validates the decoded format identifiers" {
            let results = decoded
                .iter()
                .map(|statement| validate_attestation_format_identifier(statement.format()))
                .collect::<Vec<_>>();
            let maximum_result = validate_attestation_format_identifier(maximum.format());
        }
        then "decoding preserves every value while client validation accepts the boundary and rejects invalid identifiers" {
            maximum_result.unwrap();
            for result in results {
                let error = result.unwrap_err();
                assert_protocol_error::<ResponseValidationError>(&error, |error| {
                    matches!(
                        error,
                        ResponseValidationError::InvalidAttestationFormatIdentifier(_)
                    )
                });
            }
        }
    }

    case "Accepts syntactically valid unknown authenticator extension identifiers during client response validation" {
        spec "https://www.w3.org/TR/webauthn-3/#sctn-extension-id"

        verify "a 32-octet identifier and distinct case variants remain valid even when unsupported" {
            use keyringd_fido::EncodedCborValue;
            use keyringd_fido::webauthn::AuthenticationExtensionsAuthenticatorOutputs;

            let outputs = AuthenticationExtensionsAuthenticatorOutputs::default()
                .with_value(
                    "a".repeat(32),
                    EncodedCborValue::from_canonical_bytes(vec![0xf5]).unwrap(),
                )
                .with_value(
                    "futureExtension",
                    EncodedCborValue::from_canonical_bytes(vec![0xf5]).unwrap(),
                )
                .with_value(
                    "Case",
                    EncodedCborValue::from_canonical_bytes(vec![0xf5]).unwrap(),
                )
                .with_value(
                    "case",
                    EncodedCborValue::from_canonical_bytes(vec![0xf5]).unwrap(),
                );

            validate_authenticator_extension_outputs(Some(&outputs)).unwrap();
            assert_eq!(outputs.values().len(), 4);
        }
    }

    case "Rejects invalid authenticator extension identifiers during client response validation" {
        spec "https://www.w3.org/TR/webauthn-3/#sctn-extension-id"

        given "decoded signed and unsigned authenticator extension outputs with invalid identifiers" {
            use keyringd_fido::EncodedCborValue;
            use keyringd_fido::webauthn::AuthenticationExtensionsAuthenticatorOutputs;
            use sha2::{Digest, Sha256};

            fn outputs(identifier: String) -> AuthenticationExtensionsAuthenticatorOutputs {
                AuthenticationExtensionsAuthenticatorOutputs::default().with_value(
                    identifier,
                    EncodedCborValue::from_canonical_bytes(vec![0xf5]).unwrap(),
                )
            }

            fn authenticator_data(
                identifier: String,
            ) -> keyringd_fido::AuthenticatorData {
                let extensions = outputs(identifier).to_cbor_bytes().unwrap();
                let mut encoded = Sha256::digest(b"example.com").to_vec();
                encoded.push(0x81);
                encoded.extend_from_slice(&0u32.to_be_bytes());
                encoded.extend_from_slice(&extensions);
                keyringd_fido::AuthenticatorData::from_bytes(&encoded).unwrap()
            }

            let invalid_identifiers = [
                "a".repeat(33),
                "café".to_owned(),
                "bad name".to_owned(),
                "bad\u{1f}name".to_owned(),
                "bad\u{7f}name".to_owned(),
                "bad\"name".to_owned(),
                "bad\\name".to_owned(),
            ];
        }
        when "the client validates each decoded extension map" {
            let signed_results = invalid_identifiers
                .iter()
                .map(|identifier| {
                    validate_authenticator_extension_outputs(
                        authenticator_data(identifier.clone()).extensions(),
                    )
                })
                .collect::<Vec<_>>();
            let unsigned_results = invalid_identifiers
                .iter()
                .map(|identifier| {
                    let outputs = outputs(identifier.clone());
                    validate_authenticator_extension_outputs(Some(&outputs))
                })
                .collect::<Vec<_>>();
        }
        then "the raw maps decode successfully but client response validation rejects every invalid identifier" {
            for result in signed_results.into_iter().chain(unsigned_results) {
                let error = result.unwrap_err();
                assert_protocol_error::<ResponseValidationError>(&error, |error| {
                    matches!(error, ResponseValidationError::InvalidExtensionIdentifier(_))
                });
            }
        }
    }
}
