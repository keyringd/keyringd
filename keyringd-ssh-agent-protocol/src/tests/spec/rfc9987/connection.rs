use crate::test_support::pending_handler;
use std::time::Duration;

use crate::{SshAgentConfig, SshAgentConnection};
use keyringd_testkit::scenario;
use tokio::io::AsyncWriteExt;

scenario! {
    case "Reject the cut-off SSH Agent Length Prefix" {
        spec "https://www.rfc-editor.org/rfc/rfc9987.html#section-5"
        async: tokio;
        timeout: Duration::from_secs(2);

        given "Prepare an Active SSH Agent connection" {
            let (mut client, server) = tokio::io::duplex(8);
            let task = tokio::spawn(SshAgentConnection::new(server, pending_handler(), SshAgentConfig::default()).serve());
        }
        when "Client cuts off and disconnects only 2 of 4 bytes length" {
            client.write_all(&[0, 0]).await.unwrap();
            drop(client);
            let result = task.await.unwrap();
        }
        then "Report unexpectedeof due to incomplete packet length" {
            assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::UnexpectedEof);
        }
    }
}
