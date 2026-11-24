use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::{
    Message,
    http::{HeaderValue, Request, header::SEC_WEBSOCKET_PROTOCOL},
};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};
#[cfg(test)]
use tracing::Instrument;

use crate::{HybridDataChannel, HybridTunnelEndpoint, HybridWebSocketError};

/// Maintains a WebSocket connection with the HYBRID tunnel, sends, receives, and terminates a binary message.
/// Noise cryptography and interpretation of CTAP messages are left to the top.
pub struct WebSocketHybridDataChannel {
    pub(crate) socket: WebSocketStream<MaybeTlsStream<TcpStream>>,
}

impl WebSocketHybridDataChannel {
    pub async fn connect(
        endpoint: &HybridTunnelEndpoint,
        #[cfg(feature = "test-support")] test_route: Option<&crate::HybridTunnelTestRoute>,
    ) -> Result<Self, HybridWebSocketError> {
        with_connection_timeout(Self::connect_inner(
            endpoint,
            #[cfg(feature = "test-support")]
            test_route,
        ))
        .await
    }

    async fn connect_inner(
        endpoint: &HybridTunnelEndpoint,
        #[cfg(feature = "test-support")] test_route: Option<&crate::HybridTunnelTestRoute>,
    ) -> Result<Self, HybridWebSocketError> {
        const MAX_REDIRECTS: usize = 5;
        let mut url = endpoint.url();
        for redirects in 0..=MAX_REDIRECTS {
            let request = web_socket_request(&url)?;
            #[cfg(not(feature = "test-support"))]
            let connection = tokio_tungstenite::connect_async(request).await;
            #[cfg(feature = "test-support")]
            let connection = if let Some(route) = test_route {
                let requested_domain = url::Url::parse(&url)
                    .map_err(|error| HybridWebSocketError::InvalidRedirect(Some(error.into())))?;
                if requested_domain.host_str() != Some(endpoint.domain()) {
                    return Err(HybridWebSocketError::InvalidRedirect(None));
                }
                let socket = TcpStream::connect(route.address)
                    .await
                    .map_err(|error| HybridWebSocketError::web_socket(error.into()))?;
                tokio_tungstenite::client_async_tls_with_config(
                    request,
                    socket,
                    None,
                    Some(tokio_tungstenite::Connector::Rustls(route.tls.clone())),
                )
                .await
            } else {
                tokio_tungstenite::connect_async(request).await
            };
            match connection {
                Ok((socket, response)) => {
                    if response.headers().get(SEC_WEBSOCKET_PROTOCOL)
                        != Some(&HeaderValue::from_static("fido.cable"))
                    {
                        return Err(HybridWebSocketError::HttpStatus(response.status().as_u16()));
                    }
                    return Ok(Self { socket });
                }
                Err(tokio_tungstenite::tungstenite::Error::Http(response))
                    if response.status().is_redirection() =>
                {
                    if redirects == MAX_REDIRECTS {
                        return Err(HybridWebSocketError::TooManyRedirects);
                    }
                    let location = response
                        .headers()
                        .get("Location")
                        .ok_or(HybridWebSocketError::InvalidRedirect(None))?
                        .to_str()
                        .map_err(|error| {
                            HybridWebSocketError::InvalidRedirect(Some(error.into()))
                        })?;
                    url = resolve_redirect(&url, location)?;
                }
                Err(tokio_tungstenite::tungstenite::Error::Http(response)) => {
                    return Err(HybridWebSocketError::HttpStatus(response.status().as_u16()));
                }
                Err(error) => return Err(HybridWebSocketError::web_socket(error)),
            }
        }
        Err(HybridWebSocketError::TooManyRedirects)
    }
}

// Bound the entire connection attempt, including DNS, TCP, TLS, WebSocket and redirects.
// BLE discovery and subsequent CTAP requests have their own operation deadline.
async fn with_connection_timeout<T>(
    connection: impl std::future::Future<Output = Result<T, HybridWebSocketError>>,
) -> Result<T, HybridWebSocketError> {
    tokio::time::timeout(std::time::Duration::from_secs(15), connection)
        .await
        .map_err(|_| HybridWebSocketError::ConnectionTimedOut)?
}

pub(crate) fn web_socket_request(url: &str) -> Result<Request<()>, HybridWebSocketError> {
    let mut request = url
        .into_client_request()
        .map_err(HybridWebSocketError::web_socket)?;
    request.headers_mut().insert(
        SEC_WEBSOCKET_PROTOCOL,
        HeaderValue::from_static("fido.cable"),
    );
    Ok(request)
}

pub(crate) fn resolve_redirect(base: &str, location: &str) -> Result<String, HybridWebSocketError> {
    let redirected = url::Url::parse(base)
        .and_then(|base| base.join(location))
        .map_err(|error| HybridWebSocketError::InvalidRedirect(Some(error.into())))?;
    (redirected.scheme() == "wss")
        .then(|| redirected.to_string())
        .ok_or(HybridWebSocketError::InvalidRedirect(None))
}

impl HybridDataChannel for WebSocketHybridDataChannel {
    type Error = HybridWebSocketError;

    async fn send(&mut self, message: Vec<u8>) -> Result<(), Self::Error> {
        crate::util::wire_trace::trace_bytes("hybrid-websocket", "out", &message, None);
        self.socket
            .send(Message::Binary(message.into()))
            .await
            .map_err(HybridWebSocketError::web_socket)
    }

    async fn receive(&mut self) -> Result<Option<Vec<u8>>, Self::Error> {
        loop {
            let Some(message) = self.socket.next().await else {
                return Ok(None);
            };
            match message.map_err(HybridWebSocketError::web_socket)? {
                Message::Binary(bytes) => {
                    crate::util::wire_trace::trace_bytes("hybrid-websocket", "in", &bytes, None);
                    return Ok(Some(bytes.to_vec()));
                }
                Message::Ping(bytes) => self
                    .socket
                    .send(Message::Pong(bytes))
                    .await
                    .map_err(HybridWebSocketError::web_socket)?,
                Message::Pong(_) => {}
                Message::Close(_) => return Ok(None),
                _ => return Err(HybridWebSocketError::UnexpectedMessage),
            }
        }
    }

    async fn close(&mut self) -> Result<(), Self::Error> {
        self.socket
            .close(None)
            .await
            .map_err(HybridWebSocketError::web_socket)
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    keyringd_testkit::scenario! {
        group "with_connection_timeout" {
            case "Stalled tls connection times out and closes socket" {
                async: tokio;
                timeout: std::time::Duration::from_secs(60);
                verify "return the handshake timeout and observe EOF on the remote socket" {
                    tokio::time::timeout(std::time::Duration::from_secs(30), async {
                        use tokio::io::AsyncReadExt;
                        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
                        let address = listener.local_addr().unwrap();
                        let connection = tokio::spawn(
                            async move {
                                let started = std::time::Instant::now();
                                let result = with_connection_timeout(async {
                                    tokio_tungstenite::connect_async(format!("wss://{address}/"))
                                        .await
                                        .map_err(HybridWebSocketError::web_socket)
                                })
                                .await;
                                (result, started.elapsed())
                            }
                            .in_current_span(),
                        );
                        let (mut server, _) = listener.accept().await.unwrap();
                        // Receive ClientHello, but deliberately never answer the TLS handshake.
                        let mut hello = [0u8; 4096];
                        assert!(server.read(&mut hello).await.unwrap() > 0);
                        let (result, elapsed) = connection.await.unwrap();
                        assert!(matches!(
                            result,
                            Err(HybridWebSocketError::ConnectionTimedOut)
                        ));
                        assert!(elapsed >= std::time::Duration::from_secs(15));
                        // Dropping the timed-out attempt must release the live TCP connection.
                        assert_eq!(server.read(&mut hello).await.unwrap(), 0);
                    })
                    .await
                    .expect("stalled TLS connection test did not finish within 30 seconds");
                }
            }
        }
    }
}
