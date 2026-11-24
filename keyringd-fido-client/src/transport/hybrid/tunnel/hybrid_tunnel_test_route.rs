use std::net::SocketAddr;
use std::sync::Arc;

use rustls::pki_types::CertificateDer;

/// Address and TLS settings for the test tunnel.
/// Even if the destination is replaced with a test environment, TLS authenticates the tunnel domain determined by the QR.
#[derive(Clone)]
pub struct HybridTunnelTestRoute {
    pub(crate) address: SocketAddr,
    pub(crate) tls: Arc<rustls::ClientConfig>,
}

impl HybridTunnelTestRoute {
    pub fn new(address: SocketAddr, ca_certificate_der: Vec<u8>) -> Result<Self, rustls::Error> {
        let mut roots = rustls::RootCertStore::empty();
        roots.add(CertificateDer::from(ca_certificate_der))?;
        let tls = rustls::ClientConfig::builder()
            .with_root_certificates(roots)
            .with_no_client_auth();
        Ok(Self {
            address,
            tls: Arc::new(tls),
        })
    }
}
