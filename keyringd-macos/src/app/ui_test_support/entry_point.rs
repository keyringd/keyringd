//! The UI test fixture entry point.
//!
//! It composes the real product desktop — the same `MacOsDesktop`, windows,
//! components and Command Runners the product entry point builds — and only
//! substitutes the outside boundaries: the store path, the Device Key
//! service and the FIDO service. The user's Application Support directory,
//! keychain and agent socket are never touched.

use keyringd_ui_core::key_operation::InteractivePasskeyService;
use std::sync::Arc;

use anyhow::Context;
use dispatch2::DispatchQueue;
use keyringd_core::application::KeyringdApplication;
use keyringd_core::test_support::SoftwareDeviceKey;
use objc2::MainThreadMarker;
use serde_json::json;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;

use super::{MacOsDeviceKeyBackend, MacOsPasskeyBackend, ax_driver};
use crate::app::handler::UnixSocketHandler;
use crate::app::mac_os_desktop::MacOsDesktop;
use keyringd_core::persistence::FileProfileRepository;
use keyringd_core::persistence::ProfileRepository;
use keyringd_core::test_support::SaveFailureProfileRepository;
use keyringd_core::test_support::{TestFidoMode, TestPasskeyService};
use keyringd_testkit::macos_ui_protocol::{ControlCommand, ControlResponse, FixtureState};
use keyringd_ui_core::key_operation::KeyOperationCoordinator;

pub(crate) fn run() -> anyhow::Result<()> {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "warn".into())
                .add_directive(
                    "keyringd_context=info"
                        .parse()
                        .expect("valid context log directive"),
                ),
        )
        .try_init();
    keyringd_core::util::FailureDiagnostics::install();
    let arguments = super::fixture_arguments::FixtureArguments::parse(std::env::args().skip(1))?;
    let mtm = MainThreadMarker::new().context("fixture must start on the main thread")?;
    crate::system::clipboard::use_private_pasteboard(mtm);
    let state_directory = arguments
        .state_file
        .parent()
        .context("state file needs a directory")?
        .to_path_buf();
    crate::system::open_ssh::FIXTURE_SSH_CONFIG
        .set(state_directory.join(".ssh/config"))
        .map_err(|_| anyhow::anyhow!("fixture SSH configuration already initialized"))?;
    let original_socket = state_directory.join("original-agent.sock");
    crate::system::open_ssh::FIXTURE_AGENT_SOCKET
        .set(original_socket)
        .map_err(|_| anyhow::anyhow!("fixture agent socket already initialized"))?;

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    let (interaction, events) = KeyOperationCoordinator::channel();
    let store = Arc::new(SaveFailureProfileRepository::new(
        FileProfileRepository::new(Arc::new(crate::system::persistence::MacOsAtomicFile::new(
            &arguments.state_file,
        ))),
    ));
    let settings = Arc::new(keyringd_core::application::SettingStore::new(Arc::new(
        keyringd_core::persistence::FileSettingRepository::new(Arc::new(
            crate::system::persistence::MacOsAtomicFile::new(&state_directory.join("setting.json")),
        )),
    ))?);
    let fido = arguments.test_fido_mode.map(|mode| {
        let service = TestPasskeyService::new().with_interaction_handler(interaction.clone());
        service.set_mode(mode);
        Arc::new(service)
    });
    let mut remote_advertisements: Option<keyringd_fido_client::HybridAdvertisementRelay> = None;
    let usb_address = arguments.authenticator_usb;
    let passkey_backend = if let Some(usb_address) = usb_address {
        let mut hybrid_connector =
            keyringd_fido_client::backend::SystemHybridChannelConnector::default();
        // PXP advertisement relay is explicitly selected by the scenario;
        // tunnel settings alone must not enable the extension.
        if arguments.accept_remote_hybrid_advertisements {
            let relay = keyringd_fido_client::HybridAdvertisementRelay::new();
            hybrid_connector =
                keyringd_fido_client::backend::SystemHybridChannelConnector::default()
                    .remote_advertisements(relay.clone());
            match (
                std::env::var("KEYRINGD_UI_TEST_HYBRID_TUNNEL_ADDRESS"),
                std::env::var("KEYRINGD_UI_TEST_HYBRID_TUNNEL_CA"),
            ) {
                (Ok(address), Ok(ca_path)) => {
                    let route = keyringd_fido_client::HybridTunnelTestRoute::new(
                        address.parse()?,
                        std::fs::read(ca_path)?,
                    )?;
                    hybrid_connector = hybrid_connector.test_tunnel_route(route);
                }
                (Err(std::env::VarError::NotPresent), Err(std::env::VarError::NotPresent)) => {}
                _ => anyhow::bail!("both Hybrid test tunnel address and CA are required"),
            }
            remote_advertisements = Some(relay);
        }
        let client = Arc::new(InteractivePasskeyService::new(
            keyringd_fido_client::FidoClient::new(interaction.clone())
                .with_usb_backend(keyringd_fido_client::backend::UsbBackend::Hid(
                    std::sync::Arc::new(
                        keyringd_test_usbip::usbip_hid_backend::UsbIpHidBackend::new([usb_address]),
                    ),
                ))
                .with_hybrid_connector(hybrid_connector),
            interaction.clone(),
        ));
        MacOsPasskeyBackend::Native(client)
    } else {
        MacOsPasskeyBackend::Test(
            Arc::clone(fido.as_ref().context("test FIDO mode is required")?),
            interaction.clone(),
        )
    };
    let application = KeyringdApplication::new(
        MacOsDeviceKeyBackend::Test(
            if std::env::var_os("KEYRINGD_UI_TEST_DEVICE_KEY_UNAVAILABLE").is_some() {
                SoftwareDeviceKey::unavailable()
            } else {
                SoftwareDeviceKey::generating()
            },
        ),
        passkey_backend,
        store.clone(),
    )?;

    // A case-private agent socket, short enough for the sun_path limit and
    // never the user's own.
    let socket_path = state_directory.join("agent.sock");
    let socket = {
        let _guard = runtime.enter();
        UnixSocketHandler::bind_fixture(&socket_path, remote_advertisements)?
    };
    let agent_socket = socket.path().display().to_string();

    let listener = runtime.block_on(TcpListener::bind(("127.0.0.1", 0)))?;
    let address = listener.local_addr()?;
    let control_store = Arc::clone(&store);
    let control_fido = fido.clone();
    runtime.spawn(async move {
        while let Ok((stream, _)) = listener.accept().await {
            let store = Arc::clone(&control_store);
            let fido = control_fido.clone();
            let agent_socket = agent_socket.clone();
            tokio::spawn(async move {
                let mut stream = BufReader::new(stream);
                let mut line = String::new();
                if stream.read_line(&mut line).await.is_err() {
                    return;
                }
                let response =
                    match handle_command(&line, &store, fido.as_deref(), &agent_socket).await {
                        Ok(value) => value,
                        Err(error) => ControlResponse::Error {
                            message: format!("{error:#}"),
                        },
                    };
                let _ = stream
                    .get_mut()
                    .write_all(
                        format!(
                            "{}\n",
                            serde_json::to_string(&response)
                                .expect("the control response should serialize because it contains only JSON-compatible fields")
                        )
                        .as_bytes(),
                    )
                    .await;
            });
        }
    });
    // Publishes the listener. The test waits for a Profiles lookup through the
    // AppKit main queue before treating the fixture as ready.
    std::fs::write(&arguments.control_file, address.to_string())?;

    MacOsDesktop::new(
        runtime,
        interaction,
        application,
        events,
        socket,
        state_directory,
        settings,
    )
    .run()
}

async fn handle_command(
    line: &str,
    store: &SaveFailureProfileRepository<FileProfileRepository>,
    fido: Option<&TestPasskeyService>,
    agent_socket: &str,
) -> anyhow::Result<ControlResponse> {
    let command: ControlCommand = serde_json::from_str(line)?;
    command.validate().map_err(anyhow::Error::msg)?;
    if let Some(operation) = command.ui {
        let result = on_main(move |mtm| ax_driver::handle(operation, mtm)).await??;
        return Ok(ControlResponse::Ui { result });
    }
    if let Some(fail) = command.fail_saves {
        store.set_save_failure(fail);
    }
    if let Some(mode) = command.fido_mode {
        fido.context("fixture uses AuthenticatorServer, not TestPasskeyService")?
            .set_mode(serde_json::from_value::<TestFidoMode>(mode)?);
    }
    if let Some(credentials) = command.fido_credentials {
        fido.context("fixture uses AuthenticatorServer, not TestPasskeyService")?
            .import_credentials(serde_json::from_value(serde_json::to_value(credentials)?)?)?;
    }
    // Product model types stay at their existing serialization boundary.
    let state: FixtureState = serde_json::from_value(json!({
        "agent_socket": agent_socket,
        "test_passkey_service": fido.is_some(),
        "requests": fido.map_or(0, TestPasskeyService::requests),
        "pending": fido.map_or(0, TestPasskeyService::pending),
        "cancelled": fido.map_or(0, TestPasskeyService::cancelled),
        "profiles": store.load()?,
        "fido_credentials": fido.map(TestPasskeyService::export_credentials).unwrap_or_default(),
    }))?;
    Ok(ControlResponse::State { state })
}

/// AppKit state may only be read from the main thread, which is inside
/// `NSApplication::run` while the control channel is being served.
async fn on_main<T, F>(work: F) -> anyhow::Result<T>
where
    T: Send + 'static,
    F: FnOnce(MainThreadMarker) -> T + Send + 'static,
{
    let (response, request) = tokio::sync::oneshot::channel();
    DispatchQueue::main().exec_async(move || {
        let mtm = MainThreadMarker::new().expect("the callback should run on the AppKit thread because it was dispatched to the GCD main queue");
        let _ = response.send(work(mtm));
    });
    tokio::time::timeout(std::time::Duration::from_secs(5), request)
        .await
        .context("AppKit main thread did not answer within 5 seconds")?
        .context("main thread work was dropped")
}
