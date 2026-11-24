use std::num::NonZeroUsize;

#[derive(Clone, Copy, Debug)]
pub struct SshAgentConfig {
    max_message_size: NonZeroUsize,
    max_pending_requests: usize,
    max_connections: NonZeroUsize,
}

impl SshAgentConfig {
    pub fn new(
        max_message_size: NonZeroUsize,
        max_pending_requests: usize,
        max_connections: NonZeroUsize,
    ) -> Self {
        Self {
            max_message_size,
            max_pending_requests,
            max_connections,
        }
    }

    pub fn with_max_message_size(mut self, max_message_size: NonZeroUsize) -> Self {
        self.max_message_size = max_message_size;
        self
    }

    pub fn with_max_pending_requests(mut self, max_pending_requests: usize) -> Self {
        self.max_pending_requests = max_pending_requests;
        self
    }

    pub fn with_max_connections(mut self, max_connections: NonZeroUsize) -> Self {
        self.max_connections = max_connections;
        self
    }

    pub fn max_message_size(self) -> usize {
        self.max_message_size.get()
    }

    pub fn max_pending_requests(self) -> usize {
        self.max_pending_requests
    }

    pub fn max_connections(self) -> usize {
        self.max_connections.get()
    }
}

impl Default for SshAgentConfig {
    fn default() -> Self {
        Self::new(
            NonZeroUsize::new(256 * 1024).expect("default message size is non-zero"),
            32,
            NonZeroUsize::new(64).expect("default connection count is non-zero"),
        )
    }
}
