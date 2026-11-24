use keyringd_ui_core::key_operation::{InteractiveAgentRequestHandler, KeyOperationCoordinator};
use std::sync::Arc;

use keyringd_core::agent::KeyringdAgentRequestHandler;
use keyringd_core::application::KeyringdApplication;
use keyringd_core::service::DeviceKeyService;
use keyringd_core::service::PasskeyService;
use keyringd_fido_client::HybridAdvertisementRelay;
use keyringd_ssh_agent_protocol::SshAgent;

use super::windows_named_pipe_listener::WindowsNamedPipeListener;
use crate::system::agent_pipe_name;

pub struct NamedPipeHandler;

impl NamedPipeHandler {
    pub async fn run<D, F>(
        agent: Arc<KeyringdApplication<D, F>>,
        device_key_context: D::OperationContext,
        coordinator: Arc<KeyOperationCoordinator>,
    ) -> std::io::Result<()>
    where
        D: DeviceKeyService + 'static,
        F: PasskeyService + 'static,
    {
        Self::run_at(agent_pipe_name()?, agent, device_key_context, coordinator).await
    }

    pub async fn run_at<D, F>(
        pipe_name: String,
        agent: Arc<KeyringdApplication<D, F>>,
        device_key_context: D::OperationContext,
        coordinator: Arc<KeyOperationCoordinator>,
    ) -> std::io::Result<()>
    where
        D: DeviceKeyService + 'static,
        F: PasskeyService + 'static,
    {
        Self::run_at_with_remote_hybrid_advertisements(
            pipe_name,
            agent,
            device_key_context,
            coordinator,
            None,
        )
        .await
    }

    pub async fn run_at_with_remote_hybrid_advertisements<D, F>(
        pipe_name: String,
        agent: Arc<KeyringdApplication<D, F>>,
        device_key_context: D::OperationContext,
        coordinator: Arc<KeyOperationCoordinator>,
        relay: Option<HybridAdvertisementRelay>,
    ) -> std::io::Result<()>
    where
        D: DeviceKeyService + 'static,
        F: PasskeyService + 'static,
    {
        tracing::info!(pipe = pipe_name, "listening on Windows named pipe");
        let listener = WindowsNamedPipeListener::new(pipe_name);
        let mut handler =
            KeyringdAgentRequestHandler::new_with_device_key_context(agent, device_key_context);
        if let Some(relay) = relay {
            handler = handler.with_remote_hybrid_advertisements(relay);
        }
        SshAgent::new(
            listener,
            InteractiveAgentRequestHandler::new(handler, coordinator),
        )
        .start()
        .await
    }
}
