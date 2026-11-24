use keyringd_ssh_agent_protocol::{SshAgentRequest, SshAgentSignFlags};

keyringd_testkit::scenario! {
    group "SSH Agent protocol representation" {
        case "Preserves an unknown request type and all request-specific bytes" {
            why "the protocol layer must preserve unsupported requests so product policy can choose the response"

            verify "an unassigned message is represented losslessly for the handler" {
                let request = SshAgentRequest::from_bytes(&[30, 0x00, 0xff, 0x80]).unwrap();
                let SshAgentRequest::Unknown { message_type, data } = request else {
                    panic!("expected an unknown request");
                };
                assert_eq!(message_type, 30);
                assert_eq!(data, [0x00, 0xff, 0x80]);
            }
        }

        case "Preserves defined, reserved, and unknown signature flag bits" {
            why "the protocol layer must retain unsupported flag bits so product policy can reject or interpret them"

            verify "known flag lookup does not discard unrecognised bits" {
                let flags = SshAgentSignFlags::from_bits(0x0000_000f);
                assert!(flags.contains(SshAgentSignFlags::RSA_SHA2_256));
                assert!(flags.contains(SshAgentSignFlags::RSA_SHA2_512));
                assert_eq!(flags.unknown_bits(), 0x0000_0009);
                assert_eq!(flags.bits(), 0x0000_000f);
            }
        }
    }
}
