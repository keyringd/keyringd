use std::path::PathBuf;
use std::sync::Arc;
use std::{io, os::windows::io::FromRawHandle, os::windows::io::OwnedHandle};

use crate::app::handler::StdioHandler;
use crate::app::ui::agent_prompt_window::AgentPromptWindow;
use crate::app::{StartupMode, WindowsApplication, WindowsDesktop};
use crate::system::device_key::CngDeviceKeyService;
use crate::system::logging;
use crate::system::webauthn::WindowsWebAuthnBackend;
use keyringd_core::persistence::FileProfileRepository;
use keyringd_fido_client::FidoClient;
use keyringd_ui_core::key_operation::KeyOperationCoordinator;
use windows::Win32::Foundation::{ERROR_ALREADY_EXISTS, GetLastError};
use windows::Win32::System::Threading::CreateMutexW;
use windows::core::PCWSTR;

pub struct WindowsEntryPoint;

impl WindowsEntryPoint {
    pub fn run() -> anyhow::Result<()> {
        let result = Self::run_inner();
        if let Err(error) = &result {
            keyringd_core::util::FailureDiagnostics::record_error("Windows entry point", error);
            // The stdio bridge is a command-line SSH transport, not a desktop launch.
            if std::env::args_os().nth(1).as_deref() != Some(std::ffi::OsStr::new("bridge")) {
                let data_dir = Self::data_dir().ok();
                if let Err(dialog_error) =
                    super::startup_failure_window::show(error, data_dir.as_deref())
                {
                    tracing::error!(?dialog_error, "could not display the startup error dialog");
                }
            }
        }
        result
    }

    fn run_inner() -> anyhow::Result<()> {
        #[cfg(feature = "test-support")]
        if std::env::args().nth(1).as_deref() == Some("--control-file")
            || (std::env::args().nth(1).is_none()
                && std::env::var_os("KEYRINGD_UI_TEST_CONTROL_FILE").is_some())
        {
            let result = crate::run_ui_test_app();
            if let Err(error) = &result
                && let Some(control_file) = std::env::var_os("KEYRINGD_UI_TEST_CONTROL_FILE")
                    .or_else(|| std::env::args_os().nth(2))
            {
                let _ = std::fs::write(
                    PathBuf::from(control_file).with_extension("error"),
                    format!("{error:#}"),
                );
            }
            return result;
        }
        let data_dir = Self::data_dir()?;
        logging::init_windows(&data_dir)?;
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()?;
        match StartupMode::parse()? {
            StartupMode::Desktop => {
                let Some(_instance_guard) = Self::acquire_desktop_instance()? else {
                    tracing::info!("another Windows agent instance is already running");
                    return Ok(());
                };
                let profile_repository = Arc::new(FileProfileRepository::new(Arc::new(
                    crate::system::persistence::WindowsAtomicFile::new(
                        &data_dir.join("profiles.json"),
                    ),
                )));
                let prompt_window = AgentPromptWindow::start()?;
                let device_key_service = CngDeviceKeyService::platform();
                let (fido_handler, fido_events) = KeyOperationCoordinator::channel();
                let usb_backend = Arc::new(WindowsWebAuthnBackend::new(Arc::clone(&prompt_window)));
                let passkey_service = FidoClient::new(fido_handler.clone())
                    .with_usb_backend(keyringd_fido_client::backend::UsbBackend::Os(usb_backend))
                    .with_hybrid_connector(
                        keyringd_fido_client::backend::SystemHybridChannelConnector::default(),
                    );
                let passkey_service =
                    keyringd_ui_core::key_operation::InteractivePasskeyService::new(
                        passkey_service,
                        fido_handler.clone(),
                    );
                runtime.block_on(async {
                    let application = Self::create_application(
                        profile_repository,
                        device_key_service,
                        passkey_service,
                    )
                    .await?;
                    WindowsDesktop::run(
                        application,
                        fido_handler,
                        fido_events,
                        prompt_window,
                        &data_dir,
                    )
                    .await
                })
            }
            StartupMode::StdioBridge => {
                runtime.block_on(async { StdioHandler::run().await.map_err(Into::into) })
            }
        }
    }

    async fn create_application(
        profile_repository: Arc<FileProfileRepository>,
        device_key_service: CngDeviceKeyService,
        passkey_service: super::NativeWindowsPasskeyBackend,
    ) -> anyhow::Result<Arc<WindowsApplication>> {
        use keyringd_core::application::KeyringdApplication;

        #[cfg(feature = "test-support")]
        let device_key_service: super::WindowsDeviceKeyBackend = device_key_service.into();
        #[cfg(feature = "test-support")]
        let passkey_service: super::WindowsPasskeyBackend = passkey_service.into();

        Ok(KeyringdApplication::new(
            device_key_service,
            passkey_service,
            profile_repository,
        )?)
    }

    fn data_dir() -> std::io::Result<PathBuf> {
        let local_app_data = std::env::var_os("LOCALAPPDATA").ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::NotFound, "LOCALAPPDATA is not set")
        })?;
        Ok(PathBuf::from(local_app_data).join("keyringd"))
    }

    fn acquire_desktop_instance() -> io::Result<Option<OwnedHandle>> {
        let sid = crate::system::current_user_sid()?;
        Self::acquire_instance(&format!(r"Local\keyringd-agent-instance-{sid}"))
    }

    fn acquire_instance(name: &str) -> io::Result<Option<OwnedHandle>> {
        let name = name
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect::<Vec<_>>();
        let handle = unsafe { CreateMutexW(None, false, PCWSTR(name.as_ptr())) }
            .map_err(io::Error::other)?;
        let already_exists = unsafe { GetLastError() } == ERROR_ALREADY_EXISTS;
        let handle = unsafe { OwnedHandle::from_raw_handle(handle.0) };
        Ok((!already_exists).then_some(handle))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    keyringd_testkit::scenario! {
        group "WindowsEntryPoint::acquire_instance" {
            case "Only one Desktop Instance can hold a mutex with the same name" {
                verify "Only one Desktop Instance can hold a mutex with the same name" {
                    let name = format!(
                        r"Local\keyringd-test-{}-{}",
                        std::process::id(),
                        std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_nanos()
                    );

                    // Given
                    let first = WindowsEntryPoint::acquire_instance(&name).unwrap().unwrap();

                    // When
                    let second = WindowsEntryPoint::acquire_instance(&name).unwrap();

                    // Then
                    assert!(second.is_none());

                    // When
                    drop(first);
                    let reacquired = WindowsEntryPoint::acquire_instance(&name).unwrap();

                    // Then
                    assert!(reacquired.is_some());
                }
            }
        }
    }
}
