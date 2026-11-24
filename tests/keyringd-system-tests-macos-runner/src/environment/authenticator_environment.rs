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

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    keyringd_testkit::scenario! {
        group "AuthenticatorEnvironment::prepare" {
            case "Virtual server starts empty and owns its tunnel" {
                ignored: "requires a JDK and the native OS USB testbed";
                verify "start with no virtual credentials and keep the Hybrid tunnel under fixture ownership" {
                    let workspace = Workspace::new(
                        Path::new(env!("CARGO_MANIFEST_DIR"))
                            .ancestors()
                            .nth(2)
                            .unwrap()
                            .to_path_buf(),
                    )
                    .unwrap();
                    let report = tempfile::tempdir().unwrap();
                    let env = AuthenticatorEnvironment::prepare(&workspace, report.path()).unwrap();
                    assert!(env.hybrid_ca.is_file());
                    let runtime = std::sync::Arc::new(
                        tokio::runtime::Builder::new_current_thread()
                            .enable_all()
                            .build()
                            .unwrap(),
                    );
                    let client =
                        keyringd_test_usbip::authenticator_server::client::AuthenticatorServerClient::new(
                            env.server.control_address(),
                        );
                    use keyringd_system_tests::authenticator::Authenticators;
                    let endpoint = format!("http://{}", env.server.control_address());
                    assert!(
                        runtime
                            .block_on(client.authenticators())
                            .unwrap()
                            .is_empty()
                    );
                    let authenticators =
                        Authenticators::connect(std::sync::Arc::clone(&runtime), &endpoint).unwrap();
                    let first = authenticators
                        .create_security_key(Default::default())
                        .unwrap();
                    let second = authenticators
                        .create_security_key(Default::default())
                        .unwrap();
                    assert_ne!(first.authenticator().id(), second.authenticator().id());
                    assert_eq!(first.clone().authenticator().id(), first.authenticator().id());
                    let phone = authenticators
                        .create_hybrid_phone(Default::default())
                        .unwrap();
                    let devices = runtime.block_on(client.authenticators()).unwrap();
                    assert_eq!(devices.len(), 3);
                    assert_eq!(devices.iter().filter(|device| device.connected).count(), 2);
                    drop(phone);
                    assert_eq!(runtime.block_on(client.authenticators()).unwrap().len(), 2);
                    drop(first);
                    drop(second);
                    drop(authenticators);
                    runtime.block_on(async {
                        client.health().await.unwrap();
                        assert!(client.authenticators().await.unwrap().is_empty());
                        let first = client
                            .create_authenticator(Default::default())
                            .await
                            .unwrap();
                        let second = client
                            .create_authenticator(Default::default())
                            .await
                            .unwrap();
                        assert_ne!(first.id(), second.id());
                        first.delete().await.unwrap();
                        second.delete().await.unwrap();
                        assert!(client.authenticators().await.unwrap().is_empty());
                    });
                }
            }
        }
    }
}
