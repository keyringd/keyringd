#[cfg(test)]
use keyringd_ui_core::key_operation::InteractivePasskeyService;
use keyringd_ui_core::key_operation::{InteractiveAgentRequestHandler, KeyOperationCoordinator};
use std::path::Path;
use std::sync::Arc;

use crate::app::MacOsApplication;
use crate::system::socket::UnixSocketListener;
use keyringd_core::agent::KeyringdAgentRequestHandler;
use keyringd_ssh_agent_protocol::SshAgent;

pub(crate) struct UnixSocketHandler {
    listener: UnixSocketListener,
    #[cfg(feature = "test-support")]
    remote_advertisements: Option<keyringd_fido_client::HybridAdvertisementRelay>,
}

impl UnixSocketHandler {
    pub(crate) fn bind(path: &Path) -> std::io::Result<Self> {
        Ok(Self {
            listener: UnixSocketListener::bind(path)?,
            #[cfg(feature = "test-support")]
            remote_advertisements: None,
        })
    }

    #[cfg(feature = "test-support")]
    pub(crate) fn bind_fixture(
        path: &Path,
        relay: Option<keyringd_fido_client::HybridAdvertisementRelay>,
    ) -> std::io::Result<Self> {
        let mut handler = Self::bind(path)?;
        handler.remote_advertisements = relay;
        Ok(handler)
    }

    pub(crate) async fn run(
        self,
        application: Arc<MacOsApplication>,
        coordinator: Arc<KeyOperationCoordinator>,
    ) -> std::io::Result<()> {
        let handler = KeyringdAgentRequestHandler::new(application);
        #[cfg(feature = "test-support")]
        let handler = if let Some(relay) = self.remote_advertisements {
            handler.with_remote_hybrid_advertisements(relay)
        } else {
            handler
        };
        SshAgent::new(
            self.listener,
            InteractiveAgentRequestHandler::new(handler, coordinator),
        )
        .start()
        .await
    }

    pub(crate) fn path(&self) -> &Path {
        self.listener.path()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::system::persistence::MacOsAtomicFile;
    use keyringd_core::application::KeyringdApplication;
    use keyringd_core::persistence::FileProfileRepository;
    use tokio::io::AsyncReadExt;

    keyringd_testkit::scenario! {
        group "UnixSocketHandler::bind" {
            case "Serves requests on a socket owned by the application" {
                async: tokio;
                timeout: std::time::Duration::from_secs(10);
                given "Prepare UNIXLISTENER/UNIX Stream" {
                    let directory = tempfile::tempdir_in("/tmp").unwrap();
                    let socket_path = directory.path().join("agent.sock");
                    let handler = UnixSocketHandler::bind(&socket_path).unwrap();
                    let mut waiting_client = std::os::unix::net::UnixStream::connect(&socket_path).unwrap();
                    use std::io::Write as _;
                    waiting_client.write_all(&[0, 0, 0, 1, 11]).unwrap();
                    waiting_client.set_nonblocking(true).unwrap();
                }
                when "Start the application-owned listener" {
                    let profiles = Arc::new(FileProfileRepository::new(Arc::new(MacOsAtomicFile::new(
                        &directory.path().join("profiles.json"),
                    ))));
                    let (interaction, _events) =
                        keyringd_ui_core::key_operation::KeyOperationCoordinator::channel();
                    let device_key_service = crate::system::device_key::MacOsDeviceKeyService::new();
                    let passkey_service = Arc::new(InteractivePasskeyService::new(
                        keyringd_fido_client::FidoClient::new(interaction.clone()).with_usb_backend(keyringd_fido_client::backend::UsbBackend::Hid(std::sync::Arc::new(
                                keyringd_fido_client::usb::HidApiBackend,
                            ))).with_hybrid_connector(keyringd_fido_client::backend::SystemHybridChannelConnector::default()),
                        interaction.clone(),
                    ));
                    #[cfg(feature = "test-support")]
                    let (device_key_service, passkey_service) = (device_key_service.into(), passkey_service.into());
                    let application = KeyringdApplication::new(device_key_service, passkey_service, profiles)
                        .unwrap();
                    let server = tokio::spawn(handler.run(application, interaction.clone()));
                    let mut client = tokio::net::UnixStream::from_std(waiting_client).unwrap();
                    let mut response = [0_u8; 9];
                    client.read_exact(&mut response).await.unwrap();
                }
                then "Serve the queued request and remove the owned socket on shutdown" {
                    assert_eq!(response, [0, 0, 0, 5, 12, 0, 0, 0, 0]);
                    server.abort();
                    server.await.unwrap_err();
                    assert!(!socket_path.exists());
                }
            }
        }
    }
}
