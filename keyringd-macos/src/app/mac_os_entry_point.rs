use keyringd_ui_core::key_operation::InteractivePasskeyService;
use std::sync::Arc;

use keyringd_core::application::KeyringdApplication;
use keyringd_fido_client::FidoClient;

use super::handler::UnixSocketHandler;
use super::mac_os_desktop::MacOsDesktop;
use crate::system::device_key::MacOsDeviceKeyService;
use keyringd_core::persistence::FileProfileRepository;
use keyringd_ui_core::key_operation::KeyOperationCoordinator;

pub(crate) struct MacOsEntryPoint;

impl MacOsEntryPoint {
    pub(crate) fn run() -> anyhow::Result<()> {
        let home =
            dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Home directory is unavailable"))?;
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()?;
        let data_dir = home.join("Library/Application Support/keyringd");
        let socket = {
            let _guard = runtime.enter();
            UnixSocketHandler::bind(&data_dir.join("agent.sock"))
        };
        let socket = match socket {
            Ok(socket) => socket,
            Err(error) if error.kind() == std::io::ErrorKind::AddrInUse => {
                return super::finder_launch::open(&home);
            }
            Err(error) => return Err(error.into()),
        };
        crate::system::MacOsLoginItem::register()?;
        let (interaction, events) = KeyOperationCoordinator::channel();
        let profile_repository = Arc::new(FileProfileRepository::new(Arc::new(
            crate::system::persistence::MacOsAtomicFile::new(&data_dir.join("profiles.json")),
        )));
        let settings = Arc::new(keyringd_core::application::SettingStore::new(Arc::new(
            keyringd_core::persistence::FileSettingRepository::new(Arc::new(
                crate::system::persistence::MacOsAtomicFile::new(&data_dir.join("setting.json")),
            )),
        ))?);
        let device_key_service = MacOsDeviceKeyService::new();
        let passkey_service = Arc::new(InteractivePasskeyService::new(
            FidoClient::new(interaction.clone())
                .with_usb_backend(keyringd_fido_client::backend::UsbBackend::Hid(
                    std::sync::Arc::new(keyringd_fido_client::usb::HidApiBackend),
                ))
                .with_hybrid_connector(
                    keyringd_fido_client::backend::SystemHybridChannelConnector::default(),
                ),
            interaction.clone(),
        ));
        #[cfg(feature = "test-support")]
        let device_key_service: crate::app::MacOsDeviceKeyBackend = device_key_service.into();
        #[cfg(feature = "test-support")]
        let passkey_service: crate::app::MacOsPasskeyBackend = passkey_service.into();
        let keyringd =
            KeyringdApplication::new(device_key_service, passkey_service, profile_repository)?;
        MacOsDesktop::new(
            runtime,
            interaction,
            keyringd,
            events,
            socket,
            home,
            settings,
        )
        .run()
    }
}
