use crate::RunnerOptions;
use anyhow::{Context, Result};
use keyringd_test_usbip::authenticator_server::manager::{
    AuthenticatorServerManager, AuthenticatorServerOptions, HybridTunnelOptions,
};
use std::{net::SocketAddr, path::PathBuf};

pub(crate) struct AuthenticatorEnvironment {
    pub usb_address: SocketAddr,
    pub control_url: String,
    pub hybrid_address: SocketAddr,
    pub hybrid_ca: PathBuf,
    _server: AuthenticatorServerManager,
}
impl AuthenticatorEnvironment {
    pub fn prepare(options: &RunnerOptions) -> Result<Self> {
        let jar = keyringd_test_usbip::authenticator_server::build::AuthenticatorServerBuild::new(
            &options.workspace_root,
        )
        .prepare()?;
        let certificate = keyringd_test_usbip::authenticator_server::certificate::HybridTestCertificate::generate(
            &options.report_directory,
        )?;
        let mut server_options = AuthenticatorServerOptions::new(
            jar,
            options.report_directory.join("authenticator.log"),
        );
        server_options.hybrid = Some(HybridTunnelOptions::from_certificate(&certificate));
        keyringd_testkit::runner::ReportRunMetadata::record_tool(
            &options.report_directory,
            keyringd_testkit::runner::ToolExecutionMetadata::capture(
                "Authenticator Java",
                &server_options.java_executable,
                &["-version"],
                &options.workspace_root,
            ),
        )?;
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        let server = runtime.block_on(AuthenticatorServerManager::start(server_options))?;
        Ok(Self {
            usb_address: server.usb_address(),
            control_url: format!("http://{}", server.control_address()),
            hybrid_address: server
                .hybrid_address()
                .context("owned server has no Hybrid tunnel")?,
            hybrid_ca: certificate.ca_certificate_der().to_owned(),
            _server: server,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    keyringd_testkit::scenario! {
        group "AuthenticatorEnvironment::prepare" {
            case "Owned server becomes healthy and stops on drop" {
                ignored: "requires a Windows JDK and free local server ports";
                verify "wait for the owned server to become healthy and shut it down on drop" {
                    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                        .ancestors()
                        .nth(2)
                        .unwrap()
                        .to_owned();
                    let report = tempfile::tempdir().unwrap();
                    let options = RunnerOptions {
                        suite: crate::TestSuite::Windows,
                        workspace_root: root,
                        report_directory: report.path().to_owned(),
                        filter: None,
                    };
                    let workspace = crate::environment::Workspace::new(&options).unwrap();
                    let _lock = crate::environment::RunLock::acquire(&workspace.cache).unwrap();
                    let environment = AuthenticatorEnvironment::prepare(&options).unwrap();
                    assert!(environment.hybrid_ca.is_file());
                    let server = &environment._server;
                    let pid = server.process_id().unwrap();
                    let runtime = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                        .unwrap();
                    runtime.block_on(server.client().health()).unwrap();
                    // Retain an independent process handle so PID reuse cannot affect verification.
                    use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
                    use windows::Win32::{
                        Foundation::WAIT_OBJECT_0,
                        System::Threading::{OpenProcess, PROCESS_SYNCHRONIZE, WaitForSingleObject},
                    };
                    let handle = unsafe { OpenProcess(PROCESS_SYNCHRONIZE, false, pid).unwrap() };
                    let handle = unsafe { OwnedHandle::from_raw_handle(handle.0) };
                    drop(environment);
                    assert_eq!(
                        unsafe {
                            WaitForSingleObject(
                                windows::Win32::Foundation::HANDLE(handle.as_raw_handle()),
                                1000,
                            )
                        },
                        WAIT_OBJECT_0
                    );
                }
            }
        }
    }
}
