use crate::transport::hybrid::tunnel::WebSocketHybridDataChannel;
use tokio::net::TcpStream;
use tokio_tungstenite::MaybeTlsStream;
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;

pub(crate) async fn connected_pair() -> (WebSocketHybridDataChannel, WebSocketStream<TcpStream>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let (client_stream, accepted) = tokio::join!(TcpStream::connect(address), listener.accept());
    let client_stream = client_stream.unwrap();
    let (server_stream, _) = accepted.unwrap();
    let request = format!("ws://{address}/cable/connect/test")
        .into_client_request()
        .unwrap();
    let (client, server) = tokio::join!(
        tokio_tungstenite::client_async(request, MaybeTlsStream::Plain(client_stream)),
        tokio_tungstenite::accept_async(server_stream),
    );
    let (client, _) = client.unwrap();
    (
        WebSocketHybridDataChannel { socket: client },
        server.unwrap(),
    )
}
