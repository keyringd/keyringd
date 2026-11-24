use crate::protocol::codec::{SshReader, SshWriter};
use crate::{SshAgentIdentity, SshAgentResponse, SshPublicKeyBlob};
use keyringd_testkit::scenario;

scenario! {
    group "SshAgentResponse::to_bytes" {
        case "Encode the SSH Agent Extension Failure response" {
            spec "https://www.rfc-editor.org/rfc/rfc9987.html#section-5.8"
            verify "Payload contains only extension failure message type" {
                assert_eq!(SshAgentResponse::ExtensionFailure.to_bytes().unwrap(), vec![28]);
            }
        }
        case "Encode the SSH Agent Identity list response" {
            spec "https://www.rfc-editor.org/rfc/rfc9987.html#section-5.5"
            given "Prepare an Identity list response containing one unsupported SSH-ED25519 Key and Comment" {
                let mut key_writer = SshWriter::new();
                key_writer.write_string(b"test-key@example.com");
                let key_blob = key_writer.into_bytes();
                let response = SshAgentResponse::IdentitiesAnswer {
                    identities: vec![SshAgentIdentity::new(SshPublicKeyBlob::from_bytes(&key_blob).unwrap(), "c1")],
                };
            }
            when "Encode the response" {
                let encoded = response.to_bytes().unwrap();
                let mut reader = SshReader::new(&encoded[1..]);
                let identity_count = reader.read_u32();
                let encoded_key = reader.read_string();
                let encoded_comment = reader.read_string();
                let finished = reader.finish();
            }
            then "Contains one identity with the original Key Blob and Comment and does not leave trailing data" {
                assert_eq!(encoded[0], 12);
                assert_eq!(identity_count.unwrap(), 1);
                assert_eq!(encoded_key.unwrap(), key_blob);
                assert_eq!(encoded_comment.unwrap(), b"c1");
                assert!(finished.is_ok());
            }
        }
    }
}
