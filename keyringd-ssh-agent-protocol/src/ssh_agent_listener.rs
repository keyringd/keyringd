use tokio::io::{AsyncRead, AsyncWrite};

pub trait SshAgentListener: Send {
    type Stream: AsyncRead + AsyncWrite + Unpin + Send + 'static;

    fn accept(&mut self)
    -> impl std::future::Future<Output = std::io::Result<Self::Stream>> + Send;
}
