use std::future::Future;

/// A lower communication interface for sending and receiving Hybrid messages.
/// Implementations such as WebSocket are in charge of message input/output and connection termination, and Noise's encryption processing is performed by the top ranks.
pub trait HybridDataChannel: Send {
    type Error: std::error::Error + Send + Sync + 'static;

    fn send(&mut self, message: Vec<u8>) -> impl Future<Output = Result<(), Self::Error>> + Send;

    fn receive(&mut self) -> impl Future<Output = Result<Option<Vec<u8>>, Self::Error>> + Send;

    fn close(&mut self) -> impl Future<Output = Result<(), Self::Error>> + Send;
}
