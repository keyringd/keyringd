use super::{AUTHENTICATOR_CONTROL_PORT, AUTHENTICATOR_USB_ADDRESS};
use anyhow::Result;
use keyringd_test_usbip::authenticator_server::certificate::HybridTestCertificate;
use keyringd_test_usbip::authenticator_server::manager::{
    AuthenticatorServerManager, AuthenticatorServerOptions, HybridTunnelOptions,
};

pub(crate) struct AuthenticatorEnvironment {
    server: AuthenticatorServerManager,
}
impl AuthenticatorEnvironment {
    pub(crate) fn start(
        jar: &std::path::Path,
        runtime_directory: &std::path::Path,
    ) -> Result<Self> {
        let certificate = HybridTestCertificate::generate(runtime_directory)?;
        let mut options = AuthenticatorServerOptions::new(
            jar.to_path_buf(),
            runtime_directory.join("authenticator.log"),
        );
        options.usb_address = AUTHENTICATOR_USB_ADDRESS.parse()?;
        options.control_address = format!("127.0.0.1:{AUTHENTICATOR_CONTROL_PORT}").parse()?;
        let mut hybrid = HybridTunnelOptions::from_certificate(&certificate);
        hybrid.address = "127.0.0.1:8443".parse()?;
        options.hybrid = Some(hybrid);
        keyringd_testkit::runner::ReportRunMetadata::record_tool(
            runtime_directory
                .parent()
                .ok_or_else(|| anyhow::anyhow!("runtime directory has no report parent"))?,
            keyringd_testkit::runner::ToolExecutionMetadata::capture(
                "Authenticator Java",
                &options.java_executable,
                &["-version"],
                jar.parent()
                    .ok_or_else(|| anyhow::anyhow!("authenticator JAR has no parent"))?,
            ),
        )?;
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        let server = runtime.block_on(AuthenticatorServerManager::start(options))?;
        Ok(Self { server })
    }

    pub(crate) fn stop(&mut self) -> Result<()> {
        self.server.stop().map_err(Into::into)
    }
}
