use keyringd_test_usbip::authenticator_server::{
    build::AuthenticatorServerBuild,
    certificate::HybridTestCertificate,
    manager::{AuthenticatorServerManager, AuthenticatorServerOptions, HybridTunnelOptions},
};
use rustls::{ClientConfig, RootCertStore};
use tokio_tungstenite::{Connector, tungstenite::client::IntoClientRequest};

keyringd_testkit::scenario! {
    group "authenticator server certificate" {
        case "Generates ca der and private server identity without java" {
            verify "generates ca der and private server identity without java" {
                let directory = tempfile::tempdir().unwrap();
                let certificate = HybridTestCertificate::generate(directory.path()).unwrap();
                let ca = std::fs::read(certificate.ca_certificate_der()).unwrap();
                RootCertStore::empty().add(ca.into()).unwrap();
                assert_eq!(
                    std::fs::read_to_string(certificate.certificate_chain_pem())
                        .unwrap()
                        .matches("BEGIN CERTIFICATE")
                        .count(),
                    2
                );
                assert!(
                    std::fs::read_to_string(certificate.private_key_pem())
                        .unwrap()
                        .starts_with("-----BEGIN PRIVATE KEY-----")
                );
                assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 4);
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    assert_eq!(
                        std::fs::metadata(certificate.private_key_pem())
                            .unwrap()
                            .permissions()
                            .mode()
                            & 0o777,
                        0o600
                    );
                }
            }
        }
    }
}

keyringd_testkit::scenario! {
    group "authenticator server certificate" {
        case "Rust generated identity is loaded by jvm and verified by rustls" {
            async: tokio;
            ignored: "requires a JDK and local listener ports";
            verify "rust generated identity is loaded by jvm and verified by rustls" {
                let directory = tempfile::tempdir().unwrap();
                let certificate = HybridTestCertificate::generate(directory.path()).unwrap();
                let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                    .ancestors()
                    .nth(2)
                    .unwrap();
                let jar = AuthenticatorServerBuild::new(root).prepare().unwrap();
                let mut options = AuthenticatorServerOptions::new(jar, directory.path().join("server.log"));
                options.hybrid = Some(HybridTunnelOptions::from_certificate(&certificate));
                let mut server = AuthenticatorServerManager::start(options).await.unwrap();
                let address = server.hybrid_address().unwrap();
                let ca = std::fs::read(certificate.ca_certificate_der()).unwrap();
                assert!(connect(address, "cable.ua5v.com", ca.clone()).await.is_ok());
                // The same listener must reject a different CA and an uncovered hostname.
                let other_directory = tempfile::tempdir().unwrap();
                let other = HybridTestCertificate::generate(other_directory.path()).unwrap();
                assert!(
                    connect(
                        address,
                        "cable.ua5v.com",
                        std::fs::read(other.ca_certificate_der()).unwrap()
                    )
                    .await
                    .is_err()
                );
                assert!(connect(address, "invalid.example", ca).await.is_err());
                server.stop().unwrap();
            }
        }
    }
}

async fn connect(address: std::net::SocketAddr, hostname: &str, ca: Vec<u8>) -> anyhow::Result<()> {
    let mut roots = RootCertStore::empty();
    roots.add(ca.into())?;
    let tls = ClientConfig::builder()
        .with_root_certificates(roots)
        .with_no_client_auth();
    let mut request =
        format!("wss://{hostname}/cable/connect/aabbcc/00112233445566778899aabbccddeeff")
            .into_client_request()?;
    request
        .headers_mut()
        .insert("Sec-WebSocket-Protocol", "fido.cable".parse()?);
    let connection = async {
        let socket = tokio::net::TcpStream::connect(address).await?;
        let (_, response) = tokio_tungstenite::client_async_tls_with_config(
            request,
            socket,
            None,
            Some(Connector::Rustls(std::sync::Arc::new(tls))),
        )
        .await?;
        anyhow::ensure!(
            response
                .headers()
                .get("Sec-WebSocket-Protocol")
                .is_some_and(|v| v == "fido.cable"),
            "missing fido.cable negotiation"
        );
        Ok(())
    };
    tokio::time::timeout(std::time::Duration::from_secs(10), connection).await?
}
