use clap::Parser;
use keyringd_ui_core::key_operation::InteractivePasskeyService;

use super::{CommandLine, StartupMode};

pub(crate) struct LinuxEntryPoint;

impl LinuxEntryPoint {
    pub(crate) fn run() -> anyhow::Result<()> {
        let result = Self::run_inner();
        if let Err(error) = &result {
            keyringd_core::util::FailureDiagnostics::record_error("Linux entry point", error);
            let command = std::env::args_os().nth(1);
            if !matches!(
                command.as_deref().and_then(std::ffi::OsStr::to_str),
                Some("settings" | "relay-hybrid-advertisements")
            ) && let Err(dialog_error) = super::startup_failure_window::show(error)
            {
                tracing::error!(?dialog_error, "could not display the startup error dialog");
            }
        }
        result
    }

    fn run_inner() -> anyhow::Result<()> {
        let filter = tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
            // tss-esapi logs routine context cleanup at INFO.
            tracing_subscriber::EnvFilter::new("info,tss_esapi::context=warn")
        });
        tracing_subscriber::fmt()
            .with_env_filter(
                filter.add_directive(
                    "keyringd_context=info"
                        .parse()
                        .expect("valid context log directive"),
                ),
            )
            .try_init()
            .map_err(anyhow::Error::from_boxed)
            .map_err(|error| error.context("could not initialize logging"))?;
        keyringd_core::util::FailureDiagnostics::install();
        super::adwaita_log_filter::AdwaitaLogFilter::install();
        let command = match CommandLine::try_parse() {
            Ok(command) => command,
            Err(error) if !error.use_stderr() => {
                error.print()?;
                return Ok(());
            }
            Err(error) => return Err(error.into()),
        };
        let runtime = || {
            tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
        };
        match command.resolve()? {
            StartupMode::Agent {
                socket,
                data_dir,
                accept_remote_hybrid_advertisements,
            } => runtime()?.block_on(Self::run_agent(
                socket,
                data_dir,
                accept_remote_hybrid_advertisements,
            )),
            StartupMode::Settings => super::LinuxDesktop::open_profiles(),
            StartupMode::RelayHybridAdvertisements { agent_socket } => {
                runtime()?.block_on(super::remote_hybrid_advertisement_relay::run(&agent_socket))
            }
        }
    }

    async fn run_agent(
        socket: std::path::PathBuf,
        data_dir: std::path::PathBuf,
        accept_remote_hybrid_advertisements: bool,
    ) -> anyhow::Result<()> {
        use super::handler::UnixSocketHandler;
        let (prompt_handle, events) =
            keyringd_ui_core::key_operation::KeyOperationCoordinator::channel();
        let relay = accept_remote_hybrid_advertisements
            .then(keyringd_fido_client::HybridAdvertisementRelay::new);
        let application =
            Self::create_application(data_dir, prompt_handle.clone(), relay.clone()).await?;
        let listener =
            UnixSocketHandler::activated_or_bind(&socket)?.with_remote_hybrid_advertisements(relay);
        super::LinuxDesktop::run_agent(application, prompt_handle, events, listener)
    }

    async fn create_application(
        data_dir: std::path::PathBuf,
        prompt_handle: std::sync::Arc<keyringd_ui_core::key_operation::KeyOperationCoordinator>,
        remote_hybrid_advertisements: Option<keyringd_fido_client::HybridAdvertisementRelay>,
    ) -> anyhow::Result<std::sync::Arc<super::LinuxApplication>> {
        use std::sync::Arc;

        use keyringd_core::application::KeyringdApplication;
        use keyringd_fido_client::FidoClient;

        use crate::app::service::DeviceKeyInteraction;
        use crate::system::device_key::EsapiDeviceKeyService;
        use keyringd_core::persistence::FileProfileRepository;

        let store = Arc::new(FileProfileRepository::new(Arc::new(
            crate::system::persistence::LinuxAtomicFile::new(&data_dir.join("profiles.json")),
        )));
        let device_key_interaction: Arc<dyn DeviceKeyInteraction> = prompt_handle.clone();
        let device_key_service = EsapiDeviceKeyService::platform(Some(device_key_interaction));
        let mut hybrid_connector =
            keyringd_fido_client::backend::SystemHybridChannelConnector::default();
        if let Some(relay) = remote_hybrid_advertisements {
            let connector = keyringd_fido_client::backend::SystemHybridChannelConnector::default()
                .remote_advertisements(relay);
            hybrid_connector = connector;
        }
        let passkey_service = Arc::new(InteractivePasskeyService::new(
            FidoClient::new(prompt_handle.clone())
                .with_usb_backend(keyringd_fido_client::backend::UsbBackend::Hid(
                    std::sync::Arc::new(keyringd_fido_client::usb::HidApiBackend),
                ))
                .with_hybrid_connector(hybrid_connector),
            prompt_handle.clone(),
        ));
        Ok(KeyringdApplication::new(
            device_key_service,
            passkey_service,
            store,
        )?)
    }
}
