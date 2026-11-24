use std::time::Duration;

use keyringd_ssh_agent_protocol::{SshAgentConfig, SshAgentConnection, SshAgentRequest};

use crate::test_support::{append_string, read_frame, rejecting_handler, write_frame};

keyringd_testkit::scenario! {
    case "Returns SSH_AGENT_FAILURE for unsupported and unassigned request types" {
        why "the connection adapter must preserve a handler's generic rejection response across every request message category"
        async: tokio;
        timeout: Duration::from_secs(2);

        given "a conforming handler and all registered request types that this read-only Agent does not implement" {
            let unsupported_types = [
                17_u8, 18, 19, 20, 21, 22, 23, 25, 26,
                0, 24, 30, 240,
            ];
            let (mut client, server) = tokio::io::duplex(256);
            let task = tokio::spawn(
                SshAgentConnection::new(server, rejecting_handler(), SshAgentConfig::default())
                    .serve(),
            );
        }
        when "each unsupported type is sent as a separate request" {
            let mut responses = Vec::new();
            let mut decoded_requests = Vec::new();
            for message_type in unsupported_types {
                decoded_requests.push(SshAgentRequest::from_bytes(&[message_type]).unwrap());
                write_frame(&mut client, &[message_type]).await;
                responses.push(read_frame(&mut client).await);
            }
            drop(client);
            let result = task.await.unwrap();
        }
        then "unsupported requests preserve their type and receive generic failure" {
            for (message_type, decoded) in unsupported_types.into_iter().zip(decoded_requests) {
                assert!(matches!(
                    decoded,
                    SshAgentRequest::Unknown {
                        message_type: decoded_type,
                        data,
                    } if decoded_type == message_type && data.is_empty()
                ));
            }
            assert_eq!(responses, vec![vec![5]; unsupported_types.len()]);
            assert!(result.is_ok());
        }
    }

    case "Returns SSH_AGENT_FAILURE for an unsupported query extension" {
        why "the connection adapter must frame a handler's rejection of an extension request without interpreting extension policy"
        async: tokio;
        timeout: Duration::from_secs(2);

        given "an Agent that implements extension framing but supports no extension types" {
            let (mut client, server) = tokio::io::duplex(64);
            let task = tokio::spawn(
                SshAgentConnection::new(server, rejecting_handler(), SshAgentConfig::default())
                    .serve(),
            );
            let mut request = vec![27];
            append_string(&mut request, b"query");
        }
        when "the client sends the optional query extension" {
            write_frame(&mut client, &request).await;
            let response = read_frame(&mut client).await;
            drop(client);
            let result = task.await.unwrap();
        }
        then "the unsupported extension receives an empty generic failure" {
            assert_eq!(response, [5]);
            assert!(result.is_ok());
        }
    }
}
