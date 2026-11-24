use std::num::NonZeroUsize;
use std::time::Duration;

use keyringd_ssh_agent_protocol::{SshAgentConfig, SshAgentConnection};
use tokio::io::AsyncWriteExt;

use crate::test_support::{empty_identities, frame, read_frame, write_frame};

keyringd_testkit::scenario! {
    group "SSH Agent connection framing policy" {
        case "Returns failure for an empty message and continues on the same connection" {
            why "a malformed empty frame must not desynchronize or unnecessarily terminate an otherwise usable connection"
            async: tokio;
            timeout: Duration::from_secs(2);

            given "an Agent connection and consecutive empty and identities-request frames" {
                let (mut client, server) = tokio::io::duplex(64);
                let task = tokio::spawn(SshAgentConnection::new(server, empty_identities(), SshAgentConfig::default()).serve());
            }
            when "both frames are pipelined on one connection" {
                client.write_all(&frame(&[])).await.unwrap();
                client.write_all(&frame(&[11])).await.unwrap();
                let malformed_response = read_frame(&mut client).await;
                let identities_response = read_frame(&mut client).await;
                drop(client);
                let result = task.await.unwrap();
            }
            then "the malformed request receives failure without preventing the following request" {
                assert_eq!(malformed_response, [5]);
                assert_eq!(identities_response, [12, 0, 0, 0, 0]);
                assert!(result.is_ok());
            }
        }

        case "Accepts the configured maximum message size" {
            why "the local resource limit is inclusive so a message exactly at the configured boundary remains usable"
            async: tokio;
            timeout: Duration::from_secs(2);

            given "an Agent whose message limit is exactly one byte" {
                let (mut client, server) = tokio::io::duplex(32);
                let config = SshAgentConfig::default().with_max_message_size(NonZeroUsize::new(1).unwrap());
                let task = tokio::spawn(SshAgentConnection::new(server, empty_identities(), config).serve());
            }
            when "the client sends the one-byte identities request" {
                write_frame(&mut client, &[11]).await;
                let response = read_frame(&mut client).await;
                drop(client);
                let result = task.await.unwrap();
            }
            then "the boundary-sized message is processed normally" {
                assert_eq!(response, [12, 0, 0, 0, 0]);
                assert!(result.is_ok());
            }
        }

        case "Rejects a declared message size above the configured maximum" {
            why "the local message-size limit bounds allocation before an oversized body is read"
            async: tokio;
            timeout: Duration::from_secs(2);

            given "an Agent whose message limit is one byte" {
                let (mut client, server) = tokio::io::duplex(32);
                let config = SshAgentConfig::default().with_max_message_size(NonZeroUsize::new(1).unwrap());
                let task = tokio::spawn(SshAgentConnection::new(server, empty_identities(), config).serve());
            }
            when "the client declares a two-byte message" {
                client.write_all(&2_u32.to_be_bytes()).await.unwrap();
                let result = task.await.unwrap();
            }
            then "the connection fails before waiting for or allocating the declared body" {
                assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::InvalidData);
            }
        }
    }
}
