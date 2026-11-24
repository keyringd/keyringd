use crate::webauthn::*;

keyringd_testkit::scenario! {
    group "AuthenticationExtensionsAuthenticatorOutputs::from_cbor_bytes" {
            case "Round-trips authenticator extension outputs as a CBOR map" {
                spec "https://www.w3.org/TR/webauthn-3/#iface-authentication-extensions-authenticator-outputs"

                verify "multiple named extension values are retained" {
                    let outputs = AuthenticationExtensionsAuthenticatorOutputs::default()
                        .with_value("Case", crate::EncodedCborValue::from_canonical_bytes(vec![0xf5]).unwrap())
                        .with_value("case", crate::EncodedCborValue::from_canonical_bytes(vec![0x01]).unwrap());
                    let decoded = AuthenticationExtensionsAuthenticatorOutputs::from_cbor_bytes(
                        &outputs.to_cbor_bytes().unwrap(),
                    )
                    .unwrap();
                    assert_eq!(decoded, outputs);
                    assert_eq!(decoded.values().len(), 2);
                }
            }

            case "Rejects non-canonical authenticator extension output" {
                spec "https://www.w3.org/TR/webauthn-3/#sctn-authenticator-data"
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#message-encoding"

                given "an authenticator extension map whose integer value uses a non-minimal encoding" {
                    let encoded = hex_literal::hex!("a1 61 78 18 01");
                }
                when "the extension output is decoded from authenticator data" {
                    let result = AuthenticationExtensionsAuthenticatorOutputs::from_cbor_bytes(&encoded);
                }
                then "the non-canonical nested CBOR is rejected" {
                    assert!(result.is_err());
                }
            }
    }
}
