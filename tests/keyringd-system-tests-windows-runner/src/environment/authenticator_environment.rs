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
