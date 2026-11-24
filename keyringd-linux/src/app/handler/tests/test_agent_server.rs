use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::app::handler::UnixSocketHandler;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::system::test_support::TestApplication;

pub(super) struct TestAgentServer {
    socket_path: PathBuf,
    task: tokio::task::JoinHandle<()>,
}

impl TestAgentServer {
    pub(super) fn start(data_dir: &Path, application: Arc<TestApplication>) -> Self {
        let socket_path = data_dir.join("agent.sock");
        let server = UnixSocketHandler::bind(&socket_path).unwrap();
        let task = tokio::spawn(async move {
            server
                .run(
                    application,
                    keyringd_ui_core::key_operation::KeyOperationCoordinator::channel().0,
                    None,
                )
                .await
                .unwrap();
        });
        Self { socket_path, task }
    }

    pub(super) fn socket_path(&self) -> &Path {
        &self.socket_path
    }

    pub(super) async fn connect(&self) -> tokio::net::UnixStream {
        tokio::net::UnixStream::connect(&self.socket_path)
            .await
            .unwrap()
    }

    pub(super) async fn send_request(
        stream: &mut tokio::net::UnixStream,
        payload: &[u8],
    ) -> Vec<u8> {
        let mut request = Vec::with_capacity(4 + payload.len());
        request.extend_from_slice(&u32::try_from(payload.len()).unwrap().to_be_bytes());
        request.extend_from_slice(payload);
        stream.write_all(&request).await.unwrap();
        let mut len_buf = [0u8; 4];
        stream.read_exact(&mut len_buf).await.unwrap();
        let len = u32::from_be_bytes(len_buf) as usize;
        let mut response = vec![0u8; len];
        stream.read_exact(&mut response).await.unwrap();
        response
    }

    pub(super) fn parse_string(buf: &[u8]) -> (&[u8], &[u8]) {
        let len = u32::from_be_bytes([buf[0], buf[1], buf[2], buf[3]]) as usize;
        (&buf[4..4 + len], &buf[4 + len..])
    }
}

impl Drop for TestAgentServer {
    fn drop(&mut self) {
        self.task.abort();
    }
}
