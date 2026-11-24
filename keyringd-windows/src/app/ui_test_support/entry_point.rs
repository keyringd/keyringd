use std::sync::Arc;

use crate::app::AuthenticationPromptCommandHandler;
use crate::app::StandaloneKeyOperationPrompts;
use crate::app::handler::{GitBashHandler, NamedPipeHandler};
use crate::app::ui::about_window::AboutWindow;
use crate::app::ui::ssh_client_configuration_window::SshClientConfigurationWindow;
use crate::app::ui::tray::TrayHost;
use crate::app::ui::util::ReactorUiRuntime;
use anyhow::{Context, Result};
use keyringd_core::application::KeyringdApplication;
use keyringd_core::test_support::SoftwareDeviceKey;
use keyringd_ui_core::app_menu::AppMenuEvent;
use keyringd_ui_core::key_operation::KeyOperationCoordinator;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;

use super::ControlledOpenSsh;
use super::WindowsDeviceKeyBackend;
use super::WindowsPasskeyBackend;
use crate::app::ui::profiles_window::ProfilesWindow;
use crate::system::device_key::CngOperationContext;
use keyringd_core::persistence::FileProfileRepository;
use keyringd_core::test_support::SaveFailureProfileRepository;
use keyringd_core::test_support::TestPasskeyService;

pub(crate) fn run() -> Result<()> {
    if std::env::args().nth(1).as_deref() == Some("--cleanup-device-keys") {
        let mut args = std::env::args_os().skip(2);
        let ledger = args.next().context("missing device key ledger")?;
        anyhow::ensure!(args.next().is_none(), "unexpected cleanup argument");
        return WindowsDeviceKeyBackend::cleanup_case_keys(std::path::Path::new(&ledger));
    }
    let mut arguments = if std::env::args().nth(1).is_none() {
        vec![
            "--control-file".to_owned(),
            std::env::var("KEYRINGD_UI_TEST_CONTROL_FILE")?,
            "--state-file".to_owned(),
            std::env::var("KEYRINGD_UI_TEST_STATE_FILE")?,
            "--device-key-ledger".to_owned(),
            std::env::var("KEYRINGD_UI_TEST_DEVICE_KEY_LEDGER")?,
            "--ssh-configuration-registry-key".to_owned(),
            std::env::var("KEYRINGD_UI_TEST_SSH_CONFIGURATION_REGISTRY_KEY")?,
        ]
    } else {
        std::env::args().skip(1).collect()
    }
    .into_iter();
    anyhow::ensure!(
        arguments.next().as_deref() == Some("--control-file"),
        "--control-file is required"
    );
    let control_file = arguments.next().context("missing control file path")?;
    // A socket-activated Agent has no console. Keep its diagnostics with the
    // fixture endpoint so failures remain visible after the process restarts.
    let log_file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(std::path::Path::new(&control_file).with_extension("log"))?;
    let _ = tracing_subscriber::fmt()
        .with_ansi(false)
        .with_writer(std::sync::Mutex::new(log_file))
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "warn,keyringd_core::agent=debug".into())
                .add_directive(
                    "keyringd_context=info"
                        .parse()
                        .expect("valid context log directive"),
                ),
        )
        .try_init();
    keyringd_core::util::FailureDiagnostics::install();
    anyhow::ensure!(
        arguments.next().as_deref() == Some("--state-file"),
        "--state-file is required"
    );
    let state_file = arguments.next().context("missing state file path")?;
    std::fs::write(
        std::path::Path::new(&control_file).with_extension("pid"),
        std::process::id().to_string(),
    )?;
    anyhow::ensure!(
        arguments.next().as_deref() == Some("--device-key-ledger"),
        "--device-key-ledger is required"
    );
    let device_key_ledger =
        std::path::PathBuf::from(arguments.next().context("missing device key ledger path")?);
    anyhow::ensure!(
        arguments.next().as_deref() == Some("--ssh-configuration-registry-key"),
        "--ssh-configuration-registry-key is required"
    );
    let ssh_client_configuration_registry_key = arguments
        .next()
        .context("missing SSH configuration registry key")?;
    anyhow::ensure!(arguments.next().is_none(), "unexpected fixture argument");
    anyhow::ensure!(
        ssh_client_configuration_registry_key.starts_with(r"Software\keyringd-ui-tests\"),
        "SSH configuration registry key must be in the test namespace"
    );
    let runtime = tokio::runtime::Runtime::new()?;
    let store = Arc::new(SaveFailureProfileRepository::new(
        FileProfileRepository::new(Arc::new(
            crate::system::persistence::WindowsAtomicFile::new(std::path::Path::new(&state_file)),
        )),
    ));
    let (mock_interaction, fido_events) = KeyOperationCoordinator::channel();
    let fido =
        Arc::new(TestPasskeyService::new().with_interaction_handler(mock_interaction.clone()));
    let mut remote_advertisements = None;
    let mut webauthn_owner = None;
    let mut authentication_prompt_window = None;
    let selected_passkey_backend = std::env::var("KEYRINGD_UI_TEST_FIDO_BACKEND");
    let passkey_backend = match selected_passkey_backend.as_deref() {
        Ok("hybrid") | Err(std::env::VarError::NotPresent) => {
            let hybrid = selected_passkey_backend.as_deref() == Ok("hybrid");
            let prompt_window = crate::app::ui::agent_prompt_window::AgentPromptWindow::start()?;
            webauthn_owner = Some(prompt_window.hwnd().0 as usize);
            authentication_prompt_window = Some(Arc::clone(&prompt_window));
            let usb_backend = Arc::new(crate::system::webauthn::WindowsWebAuthnBackend::new(
                prompt_window,
            ));
            let mut hybrid_connector =
                keyringd_fido_client::backend::SystemHybridChannelConnector::default();
            if hybrid {
                let relay = keyringd_fido_client::HybridAdvertisementRelay::new();
                let mut connector =
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
                        connector = connector.test_tunnel_route(route);
                    }
                    (Err(std::env::VarError::NotPresent), Err(std::env::VarError::NotPresent)) => {}
                    _ => anyhow::bail!("both Hybrid test tunnel address and CA are required"),
                }
                hybrid_connector = connector;
                remote_advertisements = Some(relay);
            }
            WindowsPasskeyBackend::Native(Box::new(
                keyringd_ui_core::key_operation::InteractivePasskeyService::new(
                    keyringd_fido_client::FidoClient::new(mock_interaction.clone())
                        .with_usb_backend(keyringd_fido_client::backend::UsbBackend::Os(
                            usb_backend,
                        ))
                        .with_hybrid_connector(hybrid_connector),
                    mock_interaction.clone(),
                ),
            ))
        }
        Ok("test-service") => {
            WindowsPasskeyBackend::Test(Arc::clone(&fido), mock_interaction.clone())
        }
        value => anyhow::bail!("invalid UI test FIDO backend: {value:?}"),
    };
    let uses_test_passkey_service = matches!(&passkey_backend, WindowsPasskeyBackend::Test(..));
    let selected_device_key_backend = std::env::var("KEYRINGD_UI_TEST_DEVICE_KEY_BACKEND");
    let cng_prompt_window = match selected_device_key_backend.as_deref() {
        // Production shares one authentication owner between WebAuthn and CNG.
        Ok("cng-software") => Some(match authentication_prompt_window {
            Some(window) => window,
            None => crate::app::ui::agent_prompt_window::AgentPromptWindow::start()?,
        }),
        Ok("software") | Err(std::env::VarError::NotPresent) => None,
        value => anyhow::bail!("invalid UI test Device Key backend: {value:?}"),
    };
    let cng_owner = cng_prompt_window
        .as_ref()
        .map(|window| window.hwnd().0 as usize);
    let device_key_context = CngOperationContext::for_window(
        cng_prompt_window
            .as_ref()
            .map_or(0, |window| window.hwnd().0 as usize),
    );
    let device_key_backend =
        if std::env::var_os("KEYRINGD_UI_TEST_DEVICE_KEY_UNAVAILABLE").is_some() {
            WindowsDeviceKeyBackend::Test(SoftwareDeviceKey::unavailable())
        } else if cng_prompt_window.is_some() {
            WindowsDeviceKeyBackend::for_case(device_key_ledger)?
        } else {
            WindowsDeviceKeyBackend::Test(SoftwareDeviceKey::generating())
        };
    let ssh = Arc::new(ControlledOpenSsh::new());
    let profiles_ssh = Arc::clone(&ssh);
    let application = KeyringdApplication::new(device_key_backend, passkey_backend, store.clone())?;
    let pipe = crate::system::agent_pipe_name()?;
    let agent_application = Arc::clone(&application);
    let agent_pipe = pipe.clone();
    let git_socket = std::path::Path::new(&state_file).with_file_name("agent.sock");
    let control_git_socket = git_socket.clone();
    let git_application = Arc::clone(&application);
    let fido_ui = Arc::clone(&mock_interaction);
    let menu_pipe = pipe.clone();
    let listener = runtime.block_on(TcpListener::bind(("127.0.0.1", 0)))?;
    let control_address = listener.local_addr()?.to_string();
    runtime.spawn(async move {
        while let Ok((stream, _)) = listener.accept().await {
            let store = Arc::clone(&store);
            let fido = Arc::clone(&fido);
            let mock_interaction = Arc::clone(&mock_interaction);
            let ssh = Arc::clone(&ssh);
            let pipe = pipe.clone();
            let git_socket = control_git_socket.clone();
            tokio::spawn(async move {
                let mut stream = BufReader::new(stream);
                let mut line = String::new();
                if stream.read_line(&mut line).await.is_err() {
                    return;
                }
                let result = super::ui_test_control::UiTestControl {
                    store: &store,
                    fido: &fido,
                    ssh: &ssh,
                    pipe: &pipe,
                    uses_test_passkey_service,
                    mock_interaction: &mock_interaction,
                }
                .handle(&line);
                let response = match result {
                    Ok(mut value) => {
                        value["git_for_windows_socket"] = serde_json::json!(git_socket);
                        value["pid"] = serde_json::json!(std::process::id());
                        value["webauthn_owner"] = serde_json::json!(webauthn_owner);
                        value["cng_owner"] = serde_json::json!(cng_owner);
                        value["active_fido_operations"] =
                            serde_json::json!(mock_interaction.active_operations());
                        value
                    }
                    Err(error) => serde_json::json!({"error": error.to_string()}),
                };
                let _ = stream
                    .get_mut()
                    .write_all(format!("{response}\n").as_bytes())
                    .await;
            });
        }
    });
    let profiles = Arc::clone(&application);
    let handle = runtime.handle().clone();
    let ui_runtime = ReactorUiRuntime::start()?;
    let dispatcher = ui_runtime.dispatcher();
    ui_runtime.invoke(move || {
        let prompts = AuthenticationPromptCommandHandler::new(
            dispatcher.clone(),
            handle.clone(),
            fido_ui.clone(),
        );
        handle.spawn(StandaloneKeyOperationPrompts::new(prompts, fido_events).run());
        let agent_coordinator = fido_ui.clone();
        let git_coordinator = fido_ui.clone();
        let coordinator = fido_ui;
        let about = AboutWindow::new(dispatcher.clone());
        let configure_ssh = SshClientConfigurationWindow::new(Some((
            ssh_client_configuration_registry_key,
            menu_pipe,
        )));
        {
            let dialog = configure_ssh.clone();
            handle.spawn(async move {
                let mut tray = TrayHost::start(Vec::new()).unwrap();
                while let Some(event) = tray.recv_event().await {
                    match event {
                        AppMenuEvent::ShowSshClientConfiguration => {
                            let dialog = dialog.clone();
                            tokio::task::spawn_blocking(move || dialog.open(0))
                                .await
                                .unwrap()
                                .unwrap();
                        }
                        AppMenuEvent::Quit => {
                            drop(tray);
                            std::process::exit(0);
                        }
                        AppMenuEvent::ShowAbout => {
                            about.open().unwrap();
                        }
                        _ => {}
                    }
                }
            });
        }
        Arc::new(ProfilesWindow::new(
            profiles,
            coordinator,
            handle.clone(),
            configure_ssh,
            Some(profiles_ssh),
        ))
        .show()
        .unwrap();
        // Match WindowsDesktop: accept Agent requests only after the FIDO UI
        // component is attached. A cold socket connection can immediately sign.
        handle.spawn(async move {
            if let Err(error) = NamedPipeHandler::run_at_with_remote_hybrid_advertisements(
                agent_pipe,
                agent_application,
                device_key_context,
                agent_coordinator,
                remote_advertisements,
            )
            .await
            {
                tracing::error!(%error, "test Agent failed");
                std::process::exit(1);
            }
        });
        handle.spawn(async move {
            if let Err(error) = GitBashHandler::run(
                &git_socket,
                git_application,
                device_key_context,
                git_coordinator,
            )
            .await
            {
                tracing::error!(%error, "test Git for Windows Agent failed");
                std::process::exit(1);
            }
        });
        Ok(())
    })?;
    // ProfilesWindow::show queues activation at high priority. Publish readiness on
    // the next normal-priority UI turn so that activation cannot dismiss a tray menu.
    ui_runtime.invoke(move || {
        std::fs::write(control_file, control_address)?;
        Ok(())
    })?;
    runtime.block_on(std::future::pending::<Result<()>>())
}
