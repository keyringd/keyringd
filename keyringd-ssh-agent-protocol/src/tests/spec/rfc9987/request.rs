use crate::protocol::codec::SshWriter;
use crate::{SshAgentCodecError, SshAgentRequest};
use keyringd_testkit::scenario;

fn unknown_public_key() -> Vec<u8> {
    let mut writer = SshWriter::new();
    writer.write_string(b"test-key@example.com");
    writer.write_string(b"opaque key data");
    writer.into_bytes()
}

scenario! {
    group "SshAgentRequest::from_bytes" {
        case "Decode SSH Agent Identity List Request" {
            spec "https://www.rfc-editor.org/rfc/rfc9987.html#section-5.5"
            verify "Decode a Payload containing only Message Type 11 as RequestIdentities" {
                assert!(matches!(SshAgentRequest::from_bytes(&[11]).unwrap(), SshAgentRequest::Identities));
            }
        }

        case "Decode the SSH Agent signature request" {
            spec "https://www.rfc-editor.org/rfc/rfc9987.html#section-5.6"
            given "Prepare a signature request, payload containing the public key blob, data byte column and zero flag" {
                let mut writer = SshWriter::new();
                writer.write_u8(13);
                let public_key = unknown_public_key();
                writer.write_string(&public_key);
                writer.write_string(b"data-to-sign");
                writer.write_u32(0);
                let payload = writer.into_bytes();
            }
            when "Decode Payload" { let request = SshAgentRequest::from_bytes(&payload).unwrap(); }
            then "Keep public key blobs, data byte columns and zero flags accurately" {
                let SshAgentRequest::Sign(sign) = request else { panic!("expected Sign") };
                assert_eq!(sign.key_blob().to_bytes(), public_key);
                assert_eq!(sign.data(), b"data-to-sign");
                assert_eq!(sign.flags().bits(), 0);
            }
        }

        case "Rejects Extra Data of SSH Agent Signature Request" {
            spec "https://www.rfc-editor.org/rfc/rfc9987.html#section-5.6"
            given "Add one extra byte to the end of a complete signature request" {
                let mut writer = SshWriter::new();
                writer.write_u8(13);
                writer.write_string(&unknown_public_key());
                writer.write_string(b"data-to-sign");
                writer.write_u32(0);
                let mut payload = writer.into_bytes();
                payload.push(1);
            }
            when "Decode Payload" { let request = SshAgentRequest::from_bytes(&payload); }
            then "Report Extra BYTE as Trailing Data" {
                assert!(matches!(request, Err(SshAgentCodecError::TrailingData { trailing_bytes: 1 })));
            }
        }

        case "Rejects SSH Agent Extension name that is not UTF-8" {
            spec "https://www.rfc-editor.org/rfc/rfc9987.html#section-5.8"
            given "Prepare an extension request whose name is not UTF-8" {
                let mut writer = SshWriter::new();
                writer.write_u8(27);
                writer.write_string(&[0xff]);
                let payload = writer.into_bytes();
            }
            when "Decode Payload" { let request = SshAgentRequest::from_bytes(&payload); }
            then "Reports Extension name as illegal UTF-8" {
                assert!(matches!(request, Err(SshAgentCodecError::InvalidUtf8 { field: "extension name" })));
            }
        }
    }
}
