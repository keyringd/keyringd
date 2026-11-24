use std::sync::Arc;
use std::time::Duration;

use crate::{SshAgent, SshAgentRequest, SshAgentResponse};

use crate::test_support::{MockSshAgentRequestHandler, listener, read_frame, write_frame};

keyringd_testkit::scenario! {
    case "Processes requests from multiple clients concurrently" {
        spec "https://www.rfc-editor.org/rfc/rfc9987.html#section-6"
        async: tokio;
        timeout: Duration::from_secs(2);

        given "an Agent listener with two client streams and a handler that waits for both requests" {
            let (mut first_client, first_server) = tokio::io::duplex(64);
            let (mut second_client, second_server) = tokio::io::duplex(64);
            let rendezvous = Arc::new(tokio::sync::Barrier::new(3));
            let mut handler = MockSshAgentRequestHandler::new();
            let handler_rendezvous = rendezvous.clone();
            handler.expect_handle().times(2).withf(|request| matches!(request, SshAgentRequest::Identities))
                .returning(move |_| {
                    let rendezvous = handler_rendezvous.clone();
                    Box::pin(async move {
                        rendezvous.wait().await;
                        SshAgentResponse::IdentitiesAnswer { identities: Vec::new() }
                    })
                });
            let listener = listener([first_server, second_server]);
            let (shutdown, shutdown_signal) = tokio::sync::oneshot::channel();
            let task = tokio::spawn(
                SshAgent::new(listener, handler).start_until(async move {
                    let _ = shutdown_signal.await;
                }),
            );
        }
        when "both clients send identities requests before either handler invocation completes" {
            write_frame(&mut first_client, &[11]).await;
            write_frame(&mut second_client, &[11]).await;
            rendezvous.wait().await;
            let first_response = read_frame(&mut first_client).await;
            let second_response = read_frame(&mut second_client).await;
            drop(first_client);
            drop(second_client);
            shutdown.send(()).unwrap();
            let result = task.await.unwrap();
        }
        then "both independent connections receive complete responses" {
            assert_eq!(first_response, [12, 0, 0, 0, 0]);
            assert_eq!(second_response, [12, 0, 0, 0, 0]);
            assert!(result.is_ok());
        }
    }
}
