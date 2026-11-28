#![cfg_attr(not(target_os = "macos"), allow(dead_code))]
use super::workspace::Workspace;
use anyhow::Result;
use std::path::Path;
use std::path::PathBuf;

use keyringd_test_usbip::authenticator_server::manager::{
    AuthenticatorServerManager, AuthenticatorServerOptions, HybridTunnelOptions,
};

pub(crate) struct AuthenticatorEnvironment {
    pub(crate) server: AuthenticatorServerManager,
    pub(crate) hybrid_ca: PathBuf,
    _directory: tempfile::TempDir,
}

/// Starts the required virtual authenticator server without pre-created keys.
impl AuthenticatorEnvironment {
    pub(crate) fn prepare(workspace: &Workspace, report_directory: &Path) -> Result<Self> {
        let jar = keyringd_test_usbip::authenticator_server::build::AuthenticatorServerBuild::new(
            workspace.root(),
        )
        .prepare()?;

        let logs = report_directory.join("authenticator-server");
        std::fs::create_dir_all(&logs)?;
        let directory = tempfile::Builder::new()
            .prefix("macos-ui-authenticator-")
            .tempdir()?;
        let certificate = keyringd_test_usbip::authenticator_server::certificate::HybridTestCertificate::generate(
            directory.path(),
        )?;
        let hybrid_ca = certificate.ca_certificate_der().to_owned();
        let mut options = AuthenticatorServerOptions::new(jar, logs.join("authenticator.log"));
        options.hybrid = Some(HybridTunnelOptions::from_certificate(&certificate));
        keyringd_testkit::runner::ReportRunMetadata::record_tool(
            report_directory,
            keyringd_testkit::runner::ToolExecutionMetadata::capture(
                "Authenticator Java",
                &options.java_executable,
                &["-version"],
                workspace.root(),
            ),
        )?;
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        let server = runtime.block_on(AuthenticatorServerManager::start(options))?;
        Ok(Self {
            server,
            hybrid_ca,
            _directory: directory,
        })
    }
}
