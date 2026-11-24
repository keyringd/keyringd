use crate::TransportError;
use crate::transport::usb::CONTINUATION_HEADER_SIZE;
use crate::transport::usb::CtapHidCommand;
use crate::transport::usb::CtapHidMessage;
use crate::transport::usb::CtapHidMessageAssembler;
use crate::transport::usb::INIT_HEADER_SIZE;
use crate::transport::usb::encode_ctap_hid_message;
use crate::transport::usb::maximum_message_size;

const TEST_PACKET_SIZE: usize = 64;

keyringd_testkit::scenario! {
    group "encode_ctap_hid_message" {
            case "Zero-fills every unused byte in CTAPHID packets" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#usb-message-and-packet-structure"

                given "one message that leaves space in its initialization packet and another that leaves space in a continuation packet" {
                    let short = CtapHidMessage::new(
                        0x01020304,
                        CtapHidCommand::Cbor,
                        vec![0xaa, 0xbb],
                    );
                    let continued = CtapHidMessage::new(
                        0x01020304,
                        CtapHidCommand::Cbor,
                        vec![0xcc; TEST_PACKET_SIZE - INIT_HEADER_SIZE + 1],
                    );
                }
                when "both messages are encoded into fixed-size HID packets" {
                    let short_packets = encode_ctap_hid_message(&short, TEST_PACKET_SIZE).unwrap();
                    let continued_packets =
                        encode_ctap_hid_message(&continued, TEST_PACKET_SIZE).unwrap();
                }
                then "all bytes after each packet payload are zero" {
                    assert!(short_packets[0][INIT_HEADER_SIZE + 2..]
                        .iter()
                        .all(|byte| *byte == 0));
                    assert_eq!(continued_packets.len(), 2);
                    assert!(continued_packets[1][CONTINUATION_HEADER_SIZE + 1..]
                        .iter()
                        .all(|byte| *byte == 0));
                }
            }
    }
}

keyringd_testkit::scenario! {
    group "encode_ctap_hid_message" {
            case "Encode an empty payload into one packet" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#usb-message-and-packet-structure"

                given "Prepare ctaphidMessage and ctaphidcommand" {
                    let message = CtapHidMessage::new(0x01020304, CtapHidCommand::Cancel, Vec::new());
                }
                when "Call encode_ctap_hid_message" {
                    let packets = encode_ctap_hid_message(&message, TEST_PACKET_SIZE).unwrap();
                }
                then "Encode an empty payload into one packet" {
                    assert_eq!(packets.len(), 1);
                    assert_eq!(&packets[0][..7], &[1, 2, 3, 4, 0x91, 0, 0]);
                }

            }
            case "Can be converted back and forth to messages of maximum length" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#usb-message-and-packet-structure"

                given "Prepare ctaphidMessage and ctaphidcommand" {
                    let message = CtapHidMessage::new(
                        0x10203040,
                        CtapHidCommand::Cbor,
                        (0..maximum_message_size(TEST_PACKET_SIZE).unwrap())
                            .map(|value| value as u8)
                            .collect(),
                    );
                }
                when "Call encode_ctap_hid_message" {
                    let packets = encode_ctap_hid_message(&message, TEST_PACKET_SIZE).unwrap();
                }
                then "Can be converted back and forth to messages of maximum length" {
                    assert_eq!(packets.len(), 129);
                    let mut assembler = CtapHidMessageAssembler::new(TEST_PACKET_SIZE).unwrap();
                    let mut decoded = None;
                    for packet in packets {
                        decoded = assembler.push(&packet).unwrap().or(decoded);
                    }
                    assert_eq!(decoded, Some(message));
                }

            }
            case "Reject messages that exceed the limit" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#usb-message-and-packet-structure"

                given "Prepare Maximum" {
                    let maximum = maximum_message_size(TEST_PACKET_SIZE).unwrap();
                }
                when "Build a CTAP HID Message that exceeds the maximum size by 1 byte" {
                    let message = CtapHidMessage::new(1, CtapHidCommand::Cbor, vec![0; maximum + 1]);
                }
                then "Verify that the results match TransportError::MessageTooLarge" {
                    assert!(matches!(
                        encode_ctap_hid_message(&message, TEST_PACKET_SIZE),
                        Err(TransportError::MessageTooLarge { .. })
                    ));
                }

            }
            case "Round-trip conversion can be done even with a Packet size other than the default value" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#usb-message-and-packet-structure"

                given "Prepare ctaphidMessage and ctaphidcommand" {
                    let message = CtapHidMessage::new(7, CtapHidCommand::Cbor, vec![0xaa; 300]);
                }
                when "Call encode_ctap_hid_message" {
                    let packets = encode_ctap_hid_message(&message, 32).unwrap();
                }
                then "Round-trip conversion can be done even with a Packet size other than the default value." {
                    assert!(packets.iter().all(|packet| packet.len() == 32));
                    let mut assembler = CtapHidMessageAssembler::new(32).unwrap();
                    let mut decoded = None;
                    for packet in packets {
                        decoded = assembler.push(&packet).unwrap().or(decoded);
                    }
                    assert_eq!(decoded, Some(message));
                }

            }
    }
}
