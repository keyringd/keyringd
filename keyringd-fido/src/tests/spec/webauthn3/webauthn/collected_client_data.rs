use crate::webauthn::*;

keyringd_testkit::scenario! {
        case "Output False in Level 3 format and omit the unspecified Top Origin" {
            spec "https://www.w3.org/TR/webauthn-3/#dictionary-client-data"
            spec "https://www.w3.org/TR/webauthn-3/#clientdatajson-serialization"

            verify "Output False in Level 3 format and omit the unspecified Top Origin" {
                let encoded = CollectedClientData::get(&[1], "https://example.com")
                    .unwrap()
                    .to_json_bytes()
                    .unwrap();
                assert_eq!(
                    std::str::from_utf8(&encoded).unwrap(),
                    r#"{"type":"webauthn.get","challenge":"AQ","origin":"https://example.com","crossOrigin":false}"#
                );
            }
        }
    group "CollectedClientData::from_json_bytes" {
            case "You can round-trip conversion of the generated Client Data Challenge" {
                spec "https://www.w3.org/TR/webauthn-3/#dictionary-client-data"

                given "Prepare CollectedClientData" {
                    let get_data = CollectedClientData::get(&[0, 1, 2, 0xff], "https://example.com").unwrap();
                    let create_data = CollectedClientData::create(&[3, 4], "https://example.com").unwrap();
                }
                when "Call data.to_json_bytes" {
                    let encoded = get_data.to_json_bytes().unwrap();
                    let parsed = CollectedClientData::from_json_bytes(&encoded).unwrap();
                    let parsed_create = CollectedClientData::from_json_bytes(
                        &create_data.to_json_bytes().unwrap()
                    ).unwrap();
                }
                then "You can round-trip conversion of the generated Client Data Challenge" {
                    assert_eq!(parsed.challenge_bytes().unwrap(), [0, 1, 2, 0xff]);
                    assert_eq!(parsed.ceremony_type(), "webauthn.get");
                    assert_eq!(parsed_create.ceremony_type(), "webauthn.create");
                    assert_eq!(parsed_create.challenge_bytes().unwrap(), [3, 4]);
                    assert_eq!(parsed.origin(), get_data.origin());
                    assert_eq!(parsed.cross_origin(), Some(false));
                }

            }
            case "Distinguish between unspecified and explicit FALSE of Cross Origin when decoding" {
                spec "https://www.w3.org/TR/webauthn-3/#dictionary-client-data"

                given "Prepare CollectedClientData" {
                    let absent = CollectedClientData::from_json_bytes(
                        br#"{"type":"webauthn.get","challenge":"AQ","origin":"https://example.com"}"#,
                    )
                    .unwrap();
                }
                when "Call CollectedClientData::from_json_bytes" {
                    let explicit = CollectedClientData::from_json_bytes(
                        br#"{"type":"webauthn.get","challenge":"AQ","origin":"https://example.com","crossOrigin":false}"#,
                    )
                    .unwrap();
                }
                then "Distinguish between unspecified and explicit FALSE of Cross Origin when decoding" {
                    assert_eq!(absent.cross_origin(), None);
                    assert_eq!(explicit.cross_origin(), Some(false));
                    assert_eq!(
                        absent.to_json_bytes().unwrap(),
                        explicit.to_json_bytes().unwrap()
                    );
                }

            }

            case "Serializes a present top origin after the cross-origin member" {
                spec "https://www.w3.org/TR/webauthn-3/#clientdatajson-serialization"

                verify "a supplied topOrigin is retained in the prescribed member order" {
                    let data = CollectedClientData::from_json_bytes(
                        br#"{"type":"webauthn.get","challenge":"AQ","origin":"https://frame.example","crossOrigin":true,"topOrigin":"https://top.example"}"#,
                    )
                    .unwrap();
                    assert_eq!(
                        data.top_origin().map(ToString::to_string).as_deref(),
                        Some("https://top.example")
                    );
                    assert_eq!(
                        std::str::from_utf8(&data.to_json_bytes().unwrap()).unwrap(),
                        r#"{"type":"webauthn.get","challenge":"AQ","origin":"https://frame.example","crossOrigin":true,"topOrigin":"https://top.example"}"#
                    );
                }
            }

            case "Reject Origin that is not in RFC 6454 format" {
                spec "https://www.w3.org/TR/webauthn-3/#dom-collectedclientdata-origin"

                verify "Origins without Scheme and Host are not accepted" {
                    for origin in [
                        "not an origin",
                        "/relative",
                        "https:///missing-host",
                        "https://example.com/path",
                        "https://example.com?query",
                        "https://example.com#fragment",
                    ] {
                        let encoded = format!(
                            r#"{{"type":"webauthn.get","challenge":"AQ","origin":"{origin}","crossOrigin":false}}"#
                        );
                        assert!(
                            CollectedClientData::from_json_bytes(encoded.as_bytes()).is_err(),
                            "accepted invalid origin: {origin}"
                        );
                        assert!(matches!(CollectedClientData::get(&[1], origin), Err(ProtocolError::InvalidOrigin(_))));
                    }
                }
            }
    }
}
