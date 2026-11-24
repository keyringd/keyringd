use std::{net::SocketAddr, path::PathBuf};

/// PEM certificate chain and PKCS#8 private key for the optional local Hybrid tunnel.
pub struct HybridTunnelOptions {
    pub address: SocketAddr,
    pub certificate_chain_pem: PathBuf,
    pub private_key_pem: PathBuf,
}
impl HybridTunnelOptions {
    pub fn from_certificate(
        certificate: &crate::authenticator_server::certificate::HybridTestCertificate,
    ) -> Self {
        Self::new(
            certificate.certificate_chain_pem().to_owned(),
            certificate.private_key_pem().to_owned(),
        )
    }
    pub fn new(certificate_chain_pem: PathBuf, private_key_pem: PathBuf) -> Self {
        Self {
            address: "127.0.0.1:0".parse().unwrap(),
            certificate_chain_pem,
            private_key_pem,
        }
    }
}
