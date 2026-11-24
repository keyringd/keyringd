use keyringd_ui_core::key_operation::{InteractiveAgentRequestHandler, KeyOperationCoordinator};
use std::path::Path;
use std::sync::Arc;

use keyringd_core::agent::KeyringdAgentRequestHandler;
use keyringd_core::application::KeyringdApplication;
use keyringd_core::service::{DeviceKeyService, PasskeyService};
use keyringd_ssh_agent_protocol::SshAgent;

use super::git_bash_socket_listener::GitBashSocketListener;

pub(crate) struct GitBashHandler;

impl GitBashHandler {
    pub(crate) async fn run<D, F>(
        path: &Path,
        application: Arc<KeyringdApplication<D, F>>,
        device_key_context: D::OperationContext,
        coordinator: Arc<KeyOperationCoordinator>,
    ) -> std::io::Result<()>
    where
        D: DeviceKeyService + 'static,
        F: PasskeyService + 'static,
    {
        let listener = GitBashSocketListener::bind(path).await?;
        tracing::info!(path = %path.display(), "listening for Git for Windows SSH clients");
        let handler = KeyringdAgentRequestHandler::new_with_device_key_context(
            application,
            device_key_context,
        );
        SshAgent::new(
            listener,
            InteractiveAgentRequestHandler::new(handler, coordinator),
        )
        .start()
        .await
    }
}
