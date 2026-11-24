use crate::SshAgentListener;
use std::future::Future;
use std::sync::Arc;

use tokio::task::JoinSet;

use crate::{SshAgentConfig, SshAgentConnection, SshAgentRequestHandler};

pub struct SshAgent<L, H> {
    listener: L,
    handler: Arc<H>,
    config: SshAgentConfig,
    active_connections: tokio::sync::watch::Sender<usize>,
}

impl<L, H> SshAgent<L, H> {
    pub fn new(listener: L, handler: H) -> Self {
        let (active_connections, _) = tokio::sync::watch::channel(0);
        Self {
            listener,
            handler: Arc::new(handler),
            config: SshAgentConfig::default(),
            active_connections,
        }
    }

    pub fn with_config(mut self, config: SshAgentConfig) -> Self {
        self.config = config;
        self
    }

    pub fn subscribe_active_connections(&self) -> tokio::sync::watch::Receiver<usize> {
        self.active_connections.subscribe()
    }
}

impl<L, H> SshAgent<L, H>
where
    L: SshAgentListener,
    H: SshAgentRequestHandler + Send + Sync + 'static,
{
    pub async fn start(self) -> std::io::Result<()> {
        self.start_until(std::future::pending()).await
    }

    pub async fn start_until<F>(mut self, shutdown: F) -> std::io::Result<()>
    where
        F: Future<Output = ()>,
    {
        let mut connections = JoinSet::new();
        tokio::pin!(shutdown);

        loop {
            tokio::select! {
                () = &mut shutdown => {
                    connections.shutdown().await;
                    let _ = self.active_connections.send(0);
                    return Ok(());
                }
                accepted = self.listener.accept(), if connections.len() < self.config.max_connections() => {
                    let stream = accepted?;
                    let handler = Arc::clone(&self.handler);
                    let config = self.config;
                    connections.spawn(async move {
                        SshAgentConnection::new(stream, handler, config).serve().await
                    });
                    let _ = self.active_connections.send(connections.len());
                }
                completed = connections.join_next(), if !connections.is_empty() => {
                    match completed {
                        Some(Ok(Ok(()))) => {}
                        Some(Ok(Err(error))) => {
                            tracing::debug!(%error, "SSH Agent connection failed");
                        }
                        Some(Err(error)) => {
                            tracing::error!(%error, "SSH Agent connection task failed");
                        }
                        None => {}
                    }
                    let _ = self.active_connections.send(connections.len());
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {

    use crate::test_support::{empty_identities, listener};
    use std::num::NonZeroUsize;
    use std::time::Duration;

    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    use super::*;

    keyringd_testkit::scenario! {
        group "SshAgent::start_until" {
            case "Waits for capacity before accepting the next connection" {
                why "To limit resource usage, do not start a new connection process beyond the number of concurrent connections you have set."
                async: tokio;
                timeout: Duration::from_secs(2);
                given "Prepare an SSH Agent with a connected Client and a waiting Client with a limited number of Active connections to 1" {
                    let (mut first_client, first_server) = tokio::io::duplex(64);
                    let (mut second_client, second_server) = tokio::io::duplex(64);
                    let listener = listener([first_server, second_server]);
                    let agent = SshAgent::new(listener, empty_identities())
                        .with_config(SshAgentConfig::default().with_max_connections(NonZeroUsize::new(1).unwrap()));
                    let (shutdown, shutdown_signal) = tokio::sync::oneshot::channel();
                    let task = tokio::spawn(agent.start_until(async move {
                        let _ = shutdown_signal.await;
                    }));
                }
                when "Both Clients request Identity while the first Client remains connected" {
                    first_client.write_all(&[0, 0, 0, 1, 11]).await.unwrap();
                    let mut response = [0_u8; 9];
                    first_client.read_exact(&mut response).await.unwrap();
                    second_client.write_all(&[0, 0, 0, 1, 11]).await.unwrap();
                    let second_response_while_full = tokio::time::timeout(
                        Duration::from_millis(25),
                        second_client.read_exact(&mut response),
                    )
                    .await;
                    drop(first_client);
                    let second_response_after_capacity = tokio::time::timeout(
                        Duration::from_secs(1),
                        second_client.read_exact(&mut response),
                    )
                    .await;
                    shutdown.send(()).unwrap();
                    let result = task.await.unwrap();
                }
                then "The next response is suspended until the first client disconnect and returns an empty Identity response after disconnection" {
                    assert!(second_response_while_full.is_err());
                    second_response_after_capacity.unwrap().unwrap();
                    assert!(result.is_ok());
                }
            }
            case "Stop the SSH Agent without issuing a Listener error when shutdown" {
                why "In order not to report the explicit Shutdown as a Listener failure and allow the caller to treat it as a normal termination."
                async: tokio;
                timeout: Duration::from_secs(2);
                given "Prepare an SSH Agent that waits for Listener's accept without a Client connection" {
                    let listener = listener([]);
                    let (shutdown, shutdown_signal) = tokio::sync::oneshot::channel();
                    let task = tokio::spawn(
                        SshAgent::new(listener, empty_identities()).start_until(async move {
                            let _ = shutdown_signal.await;
                        }),
                    );
                }
                when "Fires Agent's Cancellation Token" {
                    shutdown.send(()).unwrap();
                    let result = task.await.unwrap();
                }
                then "Successfully terminate listener loop without reporting accept error" {
                    assert!(result.is_ok());
                }
            }
        }
    }
}
