use crate::{
    TransportError,
    transport::usb::{
        CtapHidCommand, CtapHidMessage, CtapHidMessageAssembler, encode_ctap_hid_message,
    },
};

const TEST_PACKET_SIZE: usize = 64;

keyringd_testkit::scenario! {
    group "CtapHidMessageAssembler::push" {
            case "You can convert multiple Packet messages back and forth" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#usb-message-and-packet-structure"

                given "Prepare ctaphidMessage, ctaphidcommand, and ctaphidMessageAssembler" {
                    let message = CtapHidMessage::new(
                        0x01020304,
                        CtapHidCommand::Cbor,
                        (0..300).map(|value| value as u8).collect(),
                    );
                    let packets = encode_ctap_hid_message(&message, TEST_PACKET_SIZE).unwrap();
                    let mut assembler = CtapHidMessageAssembler::new(TEST_PACKET_SIZE).unwrap();
                    let mut result = None;
                }
                when "Call Assembler.push" {
                    for packet in packets {
                        result = assembler.push(&packet).unwrap().or(result);
                    }
                }
                then "You can convert multiple Packet messages back and forth" {
                    assert_eq!(result, Some(message));
                }

            }
            case "Rejects incorrect Sequence number" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#usb-message-and-packet-structure"

                given "Prepare ctaphidMessage and ctaphidcommand" {
                    let message = CtapHidMessage::new(7, CtapHidCommand::Cbor, vec![0xaa; 100]);
                    let mut packets = encode_ctap_hid_message(&message, TEST_PACKET_SIZE).unwrap();
                    packets[1][4] = 1;
                }
                when "Generate an Assembler that receives a modified Sequence number" {
                    let mut assembler = CtapHidMessageAssembler::new(TEST_PACKET_SIZE).unwrap();
                }
                then "Verify that the results match TransportError::InvalidSequence" {
                    assert_eq!(assembler.push(&packets[0]).unwrap(), None);
                    assert_eq!(
                        assembler.push(&packets[1]),
                        Err(TransportError::InvalidSequence {
                            expected: 0,
                            actual: 1,
                        })
                    );
                }

            }
            case "Reject channel changes on the way" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#usb-message-and-packet-structure"

                given "Prepare ctaphidMessage, ctaphidcommand, and ctaphidMessageAssembler" {
                    let message = CtapHidMessage::new(7, CtapHidCommand::Cbor, vec![0xaa; 100]);
                    let mut packets = encode_ctap_hid_message(&message, TEST_PACKET_SIZE).unwrap();
                    packets[1][3] = 8;
                    let mut assembler = CtapHidMessageAssembler::new(TEST_PACKET_SIZE).unwrap();
                }
                when "Call Assembler.push" {
                    assembler.push(&packets[0]).unwrap();
                }
                then "Verify that the results match TransportError::ChannelChanged" {
                    assert!(matches!(
                        assembler.push(&packets[1]),
                        Err(TransportError::ChannelChanged { .. })
                    ));
                }

            }
            case "Reject new messages when Continuation Packet is missing" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#usb-message-and-packet-structure"

                given "Prepare ctaphidMessage and ctaphidcommand" {
                    let first = encode_ctap_hid_message(
                        &CtapHidMessage::new(7, CtapHidCommand::Cbor, vec![0xaa; 100]),
                        TEST_PACKET_SIZE,
                    )
                    .unwrap();
                }
                when "Call encode_ctap_hid_message" {
                    let replacement = encode_ctap_hid_message(
                        &CtapHidMessage::new(7, CtapHidCommand::Ping, vec![1]),
                        TEST_PACKET_SIZE,
                    )
                    .unwrap();
                    let mut assembler = CtapHidMessageAssembler::new(TEST_PACKET_SIZE).unwrap();
                }
                then "Verify that the results match TransportError::UnexpectedInitialization" {
                    assert_eq!(assembler.push(&first[0]).unwrap(), None);
                    assert_eq!(
                        assembler.push(&replacement[0]),
                        Err(TransportError::UnexpectedInitialization)
                    );
                }

            }
            case "Rejects Packets with Bad Size" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#usb-message-and-packet-structure"

                verify "Deny Packets with Bad Size" {
                    let mut assembler = CtapHidMessageAssembler::new(TEST_PACKET_SIZE).unwrap();
                    assert_eq!(
                        assembler.push(&[0; 63]),
                        Err(TransportError::InvalidPacketSize {
                            expected: 64,
                            actual: 63,
                        })
                    );
                    }

            }
    }
}
