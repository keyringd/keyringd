use crate::{TransportError, transport::usb::CtapHidInitResponse};

keyringd_testkit::scenario! {
    group "CtapHidInitResponse::parse" {
            case "Parses every field of the 17-byte CTAPHID INIT response" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#usb-hid-init"

                given "an exact CTAPHID INIT response containing nonce, channel, versions, and capabilities" {
                    let payload = [
                        0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07,
                        0x11, 0x22, 0x33, 0x44,
                        0x02, 0x03, 0x04, 0x05,
                        0x05,
                    ];
                }
                when "the response is parsed" {
                    let response = CtapHidInitResponse::parse(&payload).unwrap();
                }
                then "each byte range is interpreted as its assigned field" {
                    assert_eq!(response.nonce(), &[0, 1, 2, 3, 4, 5, 6, 7]);
                    assert_eq!(response.channel_id(), 0x1122_3344);
                    assert_eq!(response.protocol_version(), 2);
                    assert_eq!(response.device_version_major(), 3);
                    assert_eq!(response.device_version_minor(), 4);
                    assert_eq!(response.device_version_build(), 5);
                    assert_eq!(response.capabilities().bits(), 0x05);
                }
            }
            case "Accepts bytes reserved for future extensions" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#usb-hid-init"

                given "Prepare Payload" {
                    let mut payload = vec![0; 17];
                    payload[8..12].copy_from_slice(&1u32.to_be_bytes());
                }
                when "Add a future expansion byte column to the end of a known field" {
                    payload.extend_from_slice(&[0xaa, 0xbb]);
                }
                then "You can decode the channel ID, ignore the trailing extension byte" {
                    assert_eq!(
                        CtapHidInitResponse::parse(&payload).unwrap().channel_id(),
                        1
                    );
                }

            }
            case "Rejects a truncated response" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#usb-hid-init"

                verify "Refused to respond cut off in the middle" {
                    assert!(matches!(
                        CtapHidInitResponse::parse(&[0; 16]),
                        Err(TransportError::InvalidInitResponseLength { actual: 16 })
                    ));
                    }

            }
    }
}
