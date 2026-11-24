use crate::test_support::{empty_identities, listener};
use std::time::Duration;

use crate::SshAgent;
use keyringd_testkit::scenario;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

scenario! {
    case "Accept Stream and handle SSH Agent connections" {
        spec "https://www.rfc-editor.org/rfc/rfc9987.html#section-6"
        async: tokio;
        timeout: Duration::from_secs(2);

        given "Prepare an SSH agent with one handler that does not return identity and one client in memory" {
            let (mut client, server) = tokio::io::duplex(64);
            let listener = listener([server]);
            let agent = SshAgent::new(listener, empty_identities());
            let (stop, shutdown) = tokio::sync::oneshot::channel();
            let task = tokio::spawn(agent.start_until(async { shutdown.await.unwrap() }));
        }
        when "Client sends ssh2_agentc_request_identities to read frame response" {
            let unsolicited = tokio::time::timeout(
                Duration::from_millis(25),
                client.read_u8(),
            )
            .await;
            client.write_all(&[0, 0, 0, 1, 11]).await.unwrap();
            let mut response = [0_u8; 9];
            client.read_exact(&mut response).await.unwrap();
            drop(client);
            stop.send(()).unwrap();
            let result = task.await.unwrap();
        }
        then "Agent returns a response of Identity number ZERO and ends with a stop request" {
            assert!(unsolicited.is_err(), "the Agent sent an unsolicited byte");
            assert_eq!(response, [0, 0, 0, 5, 12, 0, 0, 0, 0]);
            assert!(result.is_ok());
        }
    }
}
