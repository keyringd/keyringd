use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::{SshAgentConfig, SshAgentConnection, SshAgentRequest, SshAgentResponse};
use tokio::sync::Notify;

use crate::test_support::{MockSshAgentRequestHandler, read_frame, write_frame};

keyringd_testkit::scenario! {
    case "Returns pipelined responses in request order" {
        spec "https://www.rfc-editor.org/rfc/rfc9987.html#section-3"
        async: tokio;
        timeout: Duration::from_secs(2);

        given "a handler whose first request remains pending until explicitly released" {
            let calls = Arc::new(Mutex::new(Vec::new()));
            let first_started = Arc::new(Notify::new());
            let release_first = Arc::new(Notify::new());
            let mut handler = MockSshAgentRequestHandler::new();
            let mut sequence = mockall::Sequence::new();
            let first_calls = calls.clone();
            let started = first_started.clone();
            let release = release_first.clone();
            handler.expect_handle().times(1).in_sequence(&mut sequence)
                .withf(|request| matches!(request, SshAgentRequest::Unknown { message_type: 30, .. }))
                .return_once(move |_| Box::pin(async move {
                    first_calls.lock().unwrap().push(30);
                    started.notify_one();
                    release.notified().await;
                    SshAgentResponse::Success
                }));
            let second_calls = calls.clone();
            handler.expect_handle().times(1).in_sequence(&mut sequence)
                .withf(|request| matches!(request, SshAgentRequest::Unknown { message_type: 31, .. }))
                .return_once(move |_| Box::pin(async move {
                    second_calls.lock().unwrap().push(31);
                    SshAgentResponse::Failure
                }));
            let (mut client, server) = tokio::io::duplex(64);
            let task = tokio::spawn(
                SshAgentConnection::new(server, handler, SshAgentConfig::default()).serve(),
            );
        }
        when "the client pipelines message 30 followed by message 31 while message 30 is pending" {
            write_frame(&mut client, &[30]).await;
            write_frame(&mut client, &[31]).await;
            first_started.notified().await;
            let calls_while_first_is_pending = calls.lock().unwrap().clone();
            release_first.notify_one();
            let first_response = read_frame(&mut client).await;
            let second_response = read_frame(&mut client).await;
            drop(client);
            let result = task.await.unwrap();
        }
        then "the second request is dispatched only after the first and replies retain request order" {
            assert_eq!(calls_while_first_is_pending, [30]);
            assert_eq!(*calls.lock().unwrap(), [30, 31]);
            assert_eq!(first_response, [6]);
            assert_eq!(second_response, [5]);
            assert!(result.is_ok());
        }
    }
}
