use anyhow::Result;
use rcgen::{
    BasicConstraints, CertificateParams, DnType, ExtendedKeyUsagePurpose, IsCa, Issuer, KeyPair,
    KeyUsagePurpose,
};
use std::{
    fs,
    path::{Path, PathBuf},
};
use time::{Duration, OffsetDateTime};

/// Disposable TLS material. PEM is for the server and OS trust stores; DER for rustls.
#[derive(Debug)]
pub struct HybridTestCertificate {
    certificate_chain_pem: PathBuf,
    private_key_pem: PathBuf,
    ca_certificate_pem: PathBuf,
    ca_certificate_der: PathBuf,
}

impl HybridTestCertificate {
    /// Generates a seven-day test identity in Rust, without Java or external tools.
    /// The CA private key stays in memory and is discarded after signing.
    pub fn generate(directory: &Path) -> Result<Self> {
        let now = OffsetDateTime::now_utc();
        let mut ca_params = CertificateParams::new(Vec::<String>::new())?;
        ca_params
            .distinguished_name
            .push(DnType::CommonName, "keyringd-hybrid-test-ca");
        ca_params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
        ca_params.key_usages = vec![KeyUsagePurpose::KeyCertSign, KeyUsagePurpose::CrlSign];
        ca_params.not_before = now - Duration::minutes(5);
        ca_params.not_after = now + Duration::days(7);
        let ca_key = KeyPair::generate()?;
        let ca = ca_params.self_signed(&ca_key)?;
        let issuer = Issuer::new(ca_params, ca_key);
        let mut server_params =
            CertificateParams::new(vec!["cable.ua5v.com".into(), "127.0.0.1".into()])?;
        server_params
            .distinguished_name
            .push(DnType::CommonName, "cable.ua5v.com");
        server_params.key_usages = vec![KeyUsagePurpose::DigitalSignature];
        server_params.extended_key_usages = vec![ExtendedKeyUsagePurpose::ServerAuth];
        server_params.use_authority_key_identifier_extension = true;
        server_params.not_before = now - Duration::minutes(5);
        server_params.not_after = now + Duration::days(7);
        let server_key = KeyPair::generate()?;
        let server = server_params.signed_by(&server_key, &issuer)?;

        fs::create_dir_all(directory)?;
        let directory = fs::canonicalize(directory)?;
        let result = Self {
            certificate_chain_pem: directory.join("hybrid-tunnel-chain.pem"),
            private_key_pem: directory.join("hybrid-tunnel-key.pem"),
            ca_certificate_pem: directory.join("hybrid-tunnel.crt"),
            ca_certificate_der: directory.join("hybrid-tunnel-ca.der"),
        };
        // Stage the private key with restricted permissions before publishing it.
        let temporary = tempfile::Builder::new()
            .prefix("hybrid-certificate-")
            .tempdir_in(&directory)?;
        let key = temporary.path().join("key.pem");
        let mut key_file = fs::OpenOptions::new();
        key_file.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            key_file.mode(0o600);
        }
        use std::io::Write;
        key_file
            .open(&key)?
            .write_all(server_key.serialize_pem().as_bytes())?;
        for (name, contents, destination) in [
            (
                "chain.pem",
                format!("{}{}", server.pem(), ca.pem()).into_bytes(),
                &result.certificate_chain_pem,
            ),
            ("ca.pem", ca.pem().into_bytes(), &result.ca_certificate_pem),
            ("ca.der", ca.der().to_vec(), &result.ca_certificate_der),
        ] {
            let staged = temporary.path().join(name);
            fs::write(&staged, contents)?;
            fs::copy(staged, destination)?;
        }
        fs::copy(key, &result.private_key_pem)?;
        Ok(result)
    }
    pub fn certificate_chain_pem(&self) -> &Path {
        &self.certificate_chain_pem
    }
    pub fn private_key_pem(&self) -> &Path {
        &self.private_key_pem
    }
    pub fn ca_certificate_pem(&self) -> &Path {
        &self.ca_certificate_pem
    }
    pub fn ca_certificate_der(&self) -> &Path {
        &self.ca_certificate_der
    }
}
