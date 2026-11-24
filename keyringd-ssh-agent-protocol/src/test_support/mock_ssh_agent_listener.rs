use keyringd_ssh_agent_protocol::SshAgentListener;
use std::{
    collections::VecDeque,
    future::Future,
    sync::{Arc, Mutex},
};
use tokio::io::DuplexStream;

mockall::mock! {
    /// Listener mock that sets the acceptance and wait of in-memory connections.
    pub(crate) SshAgentListener {}
    impl SshAgentListener for SshAgentListener {
        type Stream = DuplexStream;
        fn accept(&mut self) -> impl Future<Output = std::io::Result<DuplexStream>> + Send;
    }
}

pub(crate) fn listener(streams: impl IntoIterator<Item = DuplexStream>) -> MockSshAgentListener {
    let streams = Arc::new(Mutex::new(streams.into_iter().collect::<VecDeque<_>>()));
    let mut listener = MockSshAgentListener::new();
    listener.expect_accept().returning(move || {
        let streams = streams.clone();
        Box::pin(async move {
            // Even if the unpolled future is discarded by SELECT!, the connection will not be taken out.
            let stream = streams.lock().unwrap().pop_front();
            match stream {
                Some(stream) => Ok(stream),
                None => std::future::pending().await,
            }
        })
    });
    listener
}
