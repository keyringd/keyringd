use std::time::Duration;

use crate::{SshAgentConfig, SshAgentConnection};
use tokio::io::AsyncWriteExt;

use crate::test_support::{append_string, empty_identities, frame, read_frame, write_frame};

keyringd_testkit::scenario! {
    group "RFC 9987 connection framing" {
        case "Reassembles a length prefix and payload split across reads" {
            spec "https://www.rfc-editor.org/rfc/rfc9987.html#section-5"
            async: tokio;
            timeout: Duration::from_secs(2);

            given "an in-memory Agent connection and the fixed identities-request frame" {
                let (mut client, server) = tokio::io::duplex(64);
                let task = tokio::spawn(
                    SshAgentConnection::new(
                        server,
                        empty_identities(),
                        SshAgentConfig::default(),
                    )
                    .serve(),
                );
                let request = frame(&[11]);
            }
            when "the client writes the four-byte length and one-byte payload in separate fragments" {
                client.write_all(&request[..1]).await.unwrap();
                client.write_all(&request[1..3]).await.unwrap();
                client.write_all(&request[3..4]).await.unwrap();
                client.write_all(&request[4..]).await.unwrap();
                let response = read_frame(&mut client).await;
                drop(client);
                let result = task.await.unwrap();
            }
            then "the Agent reconstructs one request and returns the empty identities response" {
                assert_eq!(response, [12, 0, 0, 0, 0]);
                assert!(result.is_ok());
            }
        }

        case "Reassembles request contents split across multiple body reads" {
            spec "https://www.rfc-editor.org/rfc/rfc9987.html#section-5"
            spec "https://www.rfc-editor.org/rfc/rfc9987.html#section-5.8"
            async: tokio;
            timeout: Duration::from_secs(2);

            given "an in-memory Agent connection and an extension request with a multi-byte body" {
                let (mut client, server) = tokio::io::duplex(64);
                let task = tokio::spawn(
                    SshAgentConnection::new(
                        server,
                        empty_identities(),
                        SshAgentConfig::default(),
                    )
                    .serve(),
                );
                let mut payload = vec![27];
                append_string(&mut payload, b"query");
            }
            when "the client writes the length once and splits the request body into three fragments" {
                client
                    .write_all(&u32::try_from(payload.len()).unwrap().to_be_bytes())
                    .await
                    .unwrap();
                client.write_all(&payload[..2]).await.unwrap();
                client.write_all(&payload[2..6]).await.unwrap();
                client.write_all(&payload[6..]).await.unwrap();
                let response = read_frame(&mut client).await;
                drop(client);
                let result = task.await.unwrap();
            }
            then "the complete extension reaches the handler and receives its unsupported response" {
                assert_eq!(response, [5]);
                assert!(result.is_ok());
            }
        }

        case "Rejects EOF in a framed message body" {
            spec "https://www.rfc-editor.org/rfc/rfc9987.html#section-5"
            async: tokio;
            timeout: Duration::from_secs(2);

            given "an Agent connection and a frame declaring three payload bytes" {
                let (mut client, server) = tokio::io::duplex(32);
                let task = tokio::spawn(
                    SshAgentConnection::new(
                        server,
                        empty_identities(),
                        SshAgentConfig::default(),
                    )
                    .serve(),
                );
            }
            when "the client supplies only one payload byte and disconnects" {
                client.write_all(&3_u32.to_be_bytes()).await.unwrap();
                client.write_all(&[11]).await.unwrap();
                drop(client);
                let result = task.await.unwrap();
            }
            then "the incomplete body is reported as unexpected EOF" {
                assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::UnexpectedEof);
            }
        }

        case "Returns SSH_AGENT_FAILURE on the wire for invalid requests" {
            spec "https://www.rfc-editor.org/rfc/rfc9987.html#section-5.1"
            async: tokio;
            timeout: Duration::from_secs(2);

            given "an in-memory Agent connection" {
                let (mut client, server) = tokio::io::duplex(64);
                let task = tokio::spawn(
                    SshAgentConnection::new(
                        server,
                        empty_identities(),
                        SshAgentConfig::default(),
                    )
                    .serve(),
                );
            }
            when "the client sends an unknown request and then a malformed supported request" {
                write_frame(&mut client, &[30, 0xaa, 0xbb]).await;
                let unknown_response = read_frame(&mut client).await;
                write_frame(&mut client, &[11, 0xff]).await;
                let malformed_response = read_frame(&mut client).await;
                drop(client);
                let result = task.await.unwrap();
            }
            then "each failed request receives the single-byte SSH_AGENT_FAILURE payload" {
                assert_eq!(unknown_response, [5]);
                assert_eq!(malformed_response, [5]);
                assert!(result.is_ok());
            }
        }
    }
}
