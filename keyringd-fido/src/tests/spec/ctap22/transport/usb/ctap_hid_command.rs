use crate::transport::usb::CtapHidCommand;

keyringd_testkit::scenario! {
    group "CtapHidCommand wire representation" {
            case "Maps every assigned CTAPHID command to its wire value" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#usb-commands"

                given "the CTAPHID commands assigned by the USB transport binding" {
                    let assigned = [
                        (CtapHidCommand::Ping, 0x81),
                        (CtapHidCommand::Msg, 0x83),
                        (CtapHidCommand::Lock, 0x84),
                        (CtapHidCommand::Init, 0x86),
                        (CtapHidCommand::Wink, 0x88),
                        (CtapHidCommand::Cbor, 0x90),
                        (CtapHidCommand::Cancel, 0x91),
                        (CtapHidCommand::Keepalive, 0xbb),
                        (CtapHidCommand::Error, 0xbf),
                    ];
                }
                when "the commands are encoded and decoded" {
                    let round_trips = assigned.map(|(command, wire)| {
                        (command.wire_value(), CtapHidCommand::from_wire(wire))
                    });
                }
                then "the command bit and seven-bit command identifiers match the specification" {
                    for ((command, wire), encoded, decoded) in
                        assigned.into_iter().zip(round_trips).map(|(assigned, result)| {
                            (assigned, result.0, result.1)
                        })
                    {
                        assert_eq!(encoded, wire);
                        assert_eq!(decoded, command);
                    }
                }
            }
    }
}
