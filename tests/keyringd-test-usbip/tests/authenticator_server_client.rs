use keyringd_test_usbip::authenticator_server::client::{
    AuthenticatorServerClient, AuthenticatorServerError,
};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

async fn read_headers(stream: &mut tokio::net::TcpStream) {
    let mut received = Vec::new();
    while !received.ends_with(b"\r\n\r\n") {
        received.push(stream.read_u8().await.unwrap());
    }
}

keyringd_testkit::scenario! {
    group "authenticator server client" {
        case "Control errors keep the http status and server message" {
            async: tokio;
            verify "control errors keep the http status and server message" {
                let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
                let client = AuthenticatorServerClient::new(listener.local_addr().unwrap());
                let server = async {
                    let (mut stream, _) = listener.accept().await.unwrap();
                    read_headers(&mut stream).await;
                    stream
                        .write_all(b"HTTP/1.0 409 Conflict\r\nContent-Length: 7\r\n\r\npending")
                        .await
                        .unwrap();
                };
                let (result, ()) = tokio::join!(client.health(), server);
                assert!(
                    matches!(result, Err(AuthenticatorServerError::Http { status: 409, message }) if message == "pending")
                );
            }
        }
    }
}

keyringd_testkit::scenario! {
    group "authenticator server client" {
        case "Truncated json is reported as a transport response error" {
            async: tokio;
            verify "truncated json is reported as a transport response error" {
                let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
                let client = AuthenticatorServerClient::new(listener.local_addr().unwrap());
                let server = async {
                    let (mut stream, _) = listener.accept().await.unwrap();
                    read_headers(&mut stream).await;
                    stream
                        .write_all(b"HTTP/1.0 200 OK\r\nContent-Length: 20\r\n\r\n[")
                        .await
                        .unwrap();
                };
                let (result, ()) = tokio::join!(client.authenticators(), server);
                assert!(matches!(result, Err(AuthenticatorServerError::Protocol(_))));
            }
        }
    }
}

keyringd_testkit::scenario! {
    group "authenticator server client" {
        case "An unresponsive control server does not block the test indefinitely" {
            async: tokio;
            verify "an unresponsive control server does not block the test indefinitely" {
                let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
                let client = AuthenticatorServerClient::new(listener.local_addr().unwrap())
                    .with_timeout(Duration::from_millis(50));
                let server = async {
                    let (mut stream, _) = listener.accept().await.unwrap();
                    read_headers(&mut stream).await;
                    tokio::time::sleep(Duration::from_millis(100)).await;
                };
                let (result, ()) = tokio::join!(client.health(), server);
                assert!(matches!(result, Err(AuthenticatorServerError::TimedOut)));
            }
        }
    }
}
