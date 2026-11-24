use crate::codec::CborCodec;
use crate::codec::CborItem as Value;
use crate::ctap::AuthenticatorClientPinRequest;
use crate::ctap::ClientPinSubcommand;
use crate::ctap::CtapValue;
use crate::ctap::PinUvAuthPermissions;
use crate::ctap::PinUvKeyAgreementKey;
use crate::ctap::ProtocolError;

keyringd_testkit::scenario! {
    group "AuthenticatorClientPinRequest::to_cbor_bytes" {
            case "Encodes a permission-bound PIN token request with its required fields" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#getPinUvAuthTokenUsingPinWithPermissions"

                given "a getPinUvAuthTokenUsingPinWithPermissions request bound to an RP ID" {
                    let mut sec1 = [1u8; 65];
                    sec1[0] = 4;
                    let request = AuthenticatorClientPinRequest::new(
                        ClientPinSubcommand::GetPinUvAuthTokenUsingPinWithPermissions,
                    )
                        .with_pin_uv_auth_protocol(2)
                        .with_key_agreement(PinUvKeyAgreementKey::from_sec1_public_key(&sec1).unwrap())
                        .with_pin_hash_enc(vec![7; 16])
                        .with_permissions(
                            PinUvAuthPermissions::GET_ASSERTION
                                .union(PinUvAuthPermissions::from_bits_retain(0x80)),
                        )
                        .with_relying_party_id("example.com");
                }
                when "the request is encoded and decoded" {
                    let encoded = request.to_cbor_bytes().unwrap();
                    let decoded = AuthenticatorClientPinRequest::from_cbor_bytes(&encoded).unwrap();
                    let fields = CtapValue::ctap_map(
                        CborCodec::decode(&encoded).unwrap(),
                    )
                    .unwrap();
                    let key_agreement = CborCodec::decode(
                        &request.key_agreement().unwrap().to_cbor_bytes().unwrap(),
                    )
                    .unwrap();
                }
                then "the request retains only the fields defined for that subcommand" {
                    assert_eq!(decoded, request);
                    assert_eq!(decoded.pin_uv_auth_protocol(), Some(2));
                    assert_eq!(decoded.pin_hash_enc(), Some([7; 16].as_slice()));
                    assert_eq!(
                        fields,
                        std::collections::BTreeMap::from([
                            (1, Value::Unsigned(2)),
                            (2, Value::Unsigned(9)),
                            (3, key_agreement),
                            (6, Value::ByteString(vec![7; 16])),
                            (9, Value::Unsigned(0x82)),
                            (10, Value::TextString("example.com".to_owned())),
                        ])
                    );
                }

            }
            case "Rejects a permission-bound token request without the required RP ID" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#pinuvauthtoken-permissions"

                given "a getAssertion permission request with no RP ID" {
                    let mut sec1 = [1u8; 65];
                    sec1[0] = 4;
                    let request = AuthenticatorClientPinRequest::new(
                        ClientPinSubcommand::GetPinUvAuthTokenUsingUvWithPermissions,
                    )
                        .with_pin_uv_auth_protocol(2)
                        .with_key_agreement(PinUvKeyAgreementKey::from_sec1_public_key(&sec1).unwrap())
                        .with_permissions(PinUvAuthPermissions::GET_ASSERTION);
                }
                when "the request is encoded" {
                    let result = request.to_cbor_bytes();
                }
                then "the missing permission RP ID is reported" {
                    assert!(matches!(result, Err(ProtocolError::MissingField("rpId"))));
                }
            }
            case "Rejects an empty permission set" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#perms-param-permissions"

                given "a ClientPIN request whose permission bit field is zero" {
                    let request = AuthenticatorClientPinRequest {
                        pin_uv_auth_protocol: None,
                        subcommand: ClientPinSubcommand::GetPinRetries,
                        key_agreement: None,
                        pin_uv_auth_param: None,
                        new_pin_enc: None,
                        pin_hash_enc: None,
                        permissions: Some(PinUvAuthPermissions::from_bits_retain(0)),
                        relying_party_id: None,
                    };
                }
                when "the request is encoded" {
                    let result = request.to_cbor_bytes();
                }
                then "the zero permission value is rejected" {
                    assert!(matches!(result, Err(ProtocolError::MustBePositive {
                            field: "permissions"
                        })));
                }
            }
            case "Rejects getPinToken fields that the compatibility subcommand forbids" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#getPinToken"

                given "an otherwise complete getPinToken request that also carries permissions" {
                    let mut sec1 = [1u8; 65];
                    sec1[0] = 4;
                    let request = AuthenticatorClientPinRequest::new(ClientPinSubcommand::GetPinToken)
                        .with_pin_uv_auth_protocol(1)
                        .with_key_agreement(PinUvKeyAgreementKey::from_sec1_public_key(&sec1).unwrap())
                        .with_pin_hash_enc(vec![7; 16])
                        .with_permissions(PinUvAuthPermissions::GET_ASSERTION);
                }
                when "the compatibility request is encoded" {
                    let result = request.to_cbor_bytes();
                }
                then "the forbidden permissions parameter is rejected" {
                    assert!(matches!(
                        result,
                        Err(ProtocolError::UnexpectedType {
                            field: "permissions",
                            ..
                        })
                    ));
                }
            }
            case "Rejects known ClientPIN subcommands when a mandatory parameter is missing" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#gettingSharedSecret"
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#settingNewPin"
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#changingExistingPin"
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#getPinToken"
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#getPinUvAuthTokenUsingUvWithPermissions"
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#getPinUvAuthTokenUsingPinWithPermissions"

                given "known subcommands with one specification-mandated parameter deliberately omitted" {
                    let mut sec1 = [1u8; 65];
                    sec1[0] = 4;
                    let key = PinUvKeyAgreementKey::from_sec1_public_key(&sec1).unwrap();
                    let requests = [
                        (
                            AuthenticatorClientPinRequest::new(ClientPinSubcommand::GetKeyAgreement),
                            "pinUvAuthProtocol",
                        ),
                        (
                            AuthenticatorClientPinRequest::new(ClientPinSubcommand::SetPin)
                                .with_pin_uv_auth_protocol(2)
                                .with_pin_uv_auth_param(vec![1; 32])
                                .with_new_pin_enc(vec![2; 80]),
                            "keyAgreement",
                        ),
                        (
                            AuthenticatorClientPinRequest::new(ClientPinSubcommand::ChangePin)
                                .with_pin_uv_auth_protocol(2)
                                .with_key_agreement(key.clone())
                                .with_pin_uv_auth_param(vec![1; 32])
                                .with_new_pin_enc(vec![2; 80]),
                            "pinHashEnc",
                        ),
                        (
                            AuthenticatorClientPinRequest::new(ClientPinSubcommand::GetPinToken)
                                .with_pin_uv_auth_protocol(1)
                                .with_key_agreement(key.clone()),
                            "pinHashEnc",
                        ),
                        (
                            AuthenticatorClientPinRequest::new(
                                ClientPinSubcommand::GetPinUvAuthTokenUsingUvWithPermissions,
                            )
                            .with_pin_uv_auth_protocol(2)
                            .with_key_agreement(key.clone()),
                            "permissions",
                        ),
                        (
                            AuthenticatorClientPinRequest::new(
                                ClientPinSubcommand::GetPinUvAuthTokenUsingPinWithPermissions,
                            )
                            .with_pin_uv_auth_protocol(2)
                            .with_key_agreement(key)
                            .with_pin_hash_enc(vec![3; 32]),
                            "permissions",
                        ),
                    ];
                }
                when "each incomplete request is encoded" {
                    let results = requests.map(|(request, field)| (request.to_cbor_bytes(), field));
                }
                then "each request reports the first missing mandatory parameter" {
                    for (result, field) in results {
                        assert!(matches!(result, Err(ProtocolError::MissingField(actual_field)) if actual_field == field));
                    }
                }
            }
    }
}
