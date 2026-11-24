use std::time::Duration;

use tokio::task::JoinSet;

use crate::system::WindowsBridgeRelay;
use crate::system::socket::UnixSocketListener;

const SOCKET_ACTIVATION_IDLE_TIMEOUT: Duration = Duration::from_secs(60);

pub(crate) struct WslProxyHandler;

impl WslProxyHandler {
    pub(crate) async fn run(bridge: WindowsBridgeRelay) -> anyhow::Result<()> {
        let (listener, idle_timeout) = match UnixSocketListener::from_systemd()? {
            Some(listener) => {
                tracing::info!(
                    idle_timeout_seconds = SOCKET_ACTIVATION_IDLE_TIMEOUT.as_secs(),
                    "serving systemd-activated WSL proxy socket"
                );
                (listener, Some(SOCKET_ACTIVATION_IDLE_TIMEOUT))
            }
            None => (UnixSocketListener::bind(&socket_path())?, None),
        };
        serve_listener(listener, idle_timeout, bridge).await
    }
}

async fn serve_listener(
    listener: UnixSocketListener,
    idle_timeout: Option<Duration>,
    bridge: WindowsBridgeRelay,
) -> anyhow::Result<()> {
    let mut relays = JoinSet::new();
    // When no idle timeout is configured (standalone mode), use a very long
    // sleep that never fires. The select! guard `if idle_timeout.is_some()`
    // prevents this arm from being taken.
    let mut idle = Box::pin(tokio::time::sleep(
        idle_timeout.unwrap_or(Duration::from_secs(u64::MAX / 2)),
    ));
    loop {
        tokio::select! {
            biased;
            accepted = listener.accept() => {
                let stream = accepted?;
                let bridge = bridge.clone();
                relays.spawn(async move { bridge.relay(stream).await });
            }
            completed = relays.join_next(), if !relays.is_empty() => {
                match completed {
                    Some(Ok(Ok(()))) => {}
                    Some(Ok(Err(error))) => {
                        tracing::warn!(%error, "WSL proxy connection failed");
                    }
                    Some(Err(error)) => {
                        tracing::warn!(%error, "WSL proxy connection task failed");
                    }
                    None => {}
                }
                if relays.is_empty()
                    && let Some(timeout) = idle_timeout
                {
                    idle.as_mut().reset(tokio::time::Instant::now() + timeout);
                }
            }
            () = idle.as_mut(), if idle_timeout.is_some() && relays.is_empty() => {
                tracing::info!("WSL proxy idle timeout reached");
                return Ok(());
            }
        }
    }
}

fn socket_path() -> std::path::PathBuf {
    if let Some(directory) = std::env::var_os("XDG_RUNTIME_DIR") {
        std::path::PathBuf::from(directory).join("keyringd-agent.sock")
    } else {
        let user = std::env::var("USER").unwrap_or_else(|_| "user".to_string());
        std::path::PathBuf::from(format!("/tmp/keyringd-{user}.sock"))
    }
}

#[cfg(all(test, feature = "test-support"))]
mod tests {
    use super::*;

    keyringd_testkit::scenario! {
        group "serve_listener" {
            case "Enabled Listener ends after Idle Timeout" {
                async: tokio;
                timeout: std::time::Duration::from_secs(10);
                given "There is a Unix Socket Listener with a short idle timeout" {
                    let directory = tempfile::tempdir().unwrap();
                    let listener = UnixSocketListener::bind(&directory.path().join("agent.sock")).unwrap();
                    let executable = directory.path().join("keyringd.exe");
                    std::fs::write(&executable, []).unwrap();
                }
                when "Start Listener without Client connection" {
                    let result = tokio::time::timeout(
                        Duration::from_secs(1),
                        serve_listener(
                            listener,
                            Some(Duration::from_millis(25)),
                            WindowsBridgeRelay::new(Some(executable)).unwrap(),
                        ),
                    )
                    .await;
                }
                then "Idle TIMEOUT successfully terminates" {
                    result
                        .expect("activated listener did not stop when idle")
                        .unwrap();
                }
            }
        }
    }
}
