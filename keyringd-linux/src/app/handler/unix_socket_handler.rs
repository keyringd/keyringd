use keyringd_ui_core::key_operation::{InteractiveAgentRequestHandler, KeyOperationCoordinator};
use std::path::Path;
use std::sync::Arc;

use crate::system::socket::UnixSocketListener;
use keyringd_core::agent::KeyringdAgentRequestHandler;
use keyringd_core::application::KeyringdApplication;
use keyringd_core::service::DeviceKeyService;
use keyringd_core::service::PasskeyService;
use keyringd_ssh_agent_protocol::{SshAgent, SshAgentListener};

pub(crate) struct UnixSocketHandler {
    listener: UnixSocketListener,
    remote_hybrid_advertisements: Option<keyringd_fido_client::HybridAdvertisementRelay>,
}

impl UnixSocketHandler {
    pub(crate) fn with_remote_hybrid_advertisements(
        mut self,
        relay: Option<keyringd_fido_client::HybridAdvertisementRelay>,
    ) -> Self {
        self.remote_hybrid_advertisements = relay;
        self
    }

    pub(crate) fn bind(path: &Path) -> std::io::Result<Self> {
        let listener = UnixSocketListener::bind(path)?;
        tracing::info!(socket = %path.display(), "listening on Unix socket");
        tracing::debug!(
            value = %format!("export SSH_AUTH_SOCK={}", path.display()),
            "shell environment",
        );
        Ok(Self {
            listener,
            remote_hybrid_advertisements: None,
        })
    }

    pub(crate) fn activated_or_bind(path: &Path) -> std::io::Result<Self> {
        match UnixSocketListener::from_systemd()? {
            Some(listener) => {
                tracing::info!("serving systemd-activated native agent socket");
                Ok(Self {
                    listener,
                    remote_hybrid_advertisements: None,
                })
            }
            None => Self::bind(path),
        }
    }

    pub(crate) async fn run<D, F>(
        mut self,
        agent: Arc<KeyringdApplication<D, F>>,
        coordinator: Arc<KeyOperationCoordinator>,
        activity: Option<tokio::sync::mpsc::UnboundedSender<bool>>,
    ) -> std::io::Result<()>
    where
        D: DeviceKeyService<OperationContext = ()> + 'static,
        F: PasskeyService + 'static,
    {
        let mut handler = KeyringdAgentRequestHandler::new(agent);
        if let Some(relay) = self.remote_hybrid_advertisements.take() {
            handler = handler.with_remote_hybrid_advertisements(relay);
        }
        let ssh_agent = SshAgent::new(
            self,
            InteractiveAgentRequestHandler::new(handler, coordinator),
        );
        let mut active_connections = ssh_agent.subscribe_active_connections();
        let mut server = Box::pin(ssh_agent.start());
        loop {
            tokio::select! {
                result = &mut server => return result,
                changed = active_connections.changed(), if activity.is_some() => {
                    if changed.is_err() {
                        return server.await;
                    }
                    if let Some(activity) = &activity {
                        let _ = activity.send(*active_connections.borrow_and_update() != 0);
                    }
                }
            }
        }
    }
}

impl SshAgentListener for UnixSocketHandler {
    type Stream = tokio::net::UnixStream;

    async fn accept(&mut self) -> std::io::Result<Self::Stream> {
        self.listener.accept().await
    }
}
