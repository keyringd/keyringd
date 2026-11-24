use crate::application::ApplicationError;
use crate::model::device_key::DevicePublicKey;
use crate::model::passkey::Passkey;
use crate::service::PasskeyService;
use crate::signing::fido::PasskeyAssertionProvider;
use keyringd_ssh_agent_protocol::SshCertificateFields;
use keyringd_ssh_agent_protocol::SshCertificateSigningRequest;
use keyringd_ssh_agent_protocol::{
    SshCertificate, SshCertificateBody, SshCertificateExtension, SshCertificateType,
};

const CERTIFICATE_VALID_DURATION_SECS: u64 = 100 * 365 * 24 * 60 * 60;

/// Issues Device Key certificates using Keyringd's principal, validity, and permission policy.
pub(in crate::execution) struct DeviceKeyCertificateIssuer<'a, F> {
    provider: PasskeyAssertionProvider<'a, F>,
    passkey: &'a Passkey,
}

impl<'a, F: PasskeyService> DeviceKeyCertificateIssuer<'a, F> {
    pub(in crate::execution) fn new(
        provider: PasskeyAssertionProvider<'a, F>,
        passkey: &'a Passkey,
    ) -> Self {
        Self { provider, passkey }
    }

    pub(in crate::execution) async fn issue(
        &mut self,
        public_key: &DevicePublicKey,
        application: Option<&str>,
    ) -> Result<SshCertificate, ApplicationError> {
        let request = self.prepare(public_key, application)?;
        let signature = self
            .provider
            .sign_ssh(self.passkey, request.signing_bytes())
            .await?;
        Ok(request.with_signature(signature))
    }

    pub(in crate::execution) async fn issue_and_try_write_attestation(
        &mut self,
        public_key: &DevicePublicKey,
        application: Option<&str>,
    ) -> Result<SshCertificate, ApplicationError> {
        let request = self.prepare(public_key, application)?;
        let signature = self
            .provider
            .sign_ssh_and_try_write_attestation(self.passkey, request.signing_bytes())
            .await?;
        Ok(request.with_signature(signature))
    }

    fn prepare(
        &self,
        public_key: &DevicePublicKey,
        application: Option<&str>,
    ) -> Result<SshCertificateSigningRequest, ApplicationError> {
        let public_key = public_key
            .to_ssh_public_key(application)
            .map_err(|error| ApplicationError::Certificate(error.into()))?;
        const CLOCK_SKEW_ALLOWANCE_SECS: u64 = 5 * 60;
        let principal = self.passkey.ssh_certificate_principal();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let valid_after = now.saturating_sub(CLOCK_SKEW_ALLOWANCE_SECS);
        let valid_before = now.saturating_add(CERTIFICATE_VALID_DURATION_SECS);
        let mut nonce = vec![0; 32];
        getrandom::fill(&mut nonce).expect("the operating system random source must be available");
        let mut serial = [0; 8];
        getrandom::fill(&mut serial).expect("the operating system random source must be available");
        let serial = u64::from_ne_bytes(serial);
        let mut extensions: Vec<SshCertificateExtension> = [
            "permit-X11-forwarding",
            "permit-agent-forwarding",
            "permit-port-forwarding",
            "permit-pty",
            "permit-user-rc",
        ]
        .into_iter()
        .map(SshCertificateExtension::empty)
        .collect();
        if application.is_some() {
            extensions.insert(0, SshCertificateExtension::empty("no-touch-required"));
        }
        let signature_key = self.passkey.to_ssh_public_key();
        SshCertificateBody::from_public_key(
            nonce,
            public_key,
            SshCertificateFields {
                serial,
                certificate_type: SshCertificateType::User,
                key_id: b"keyringd-tpm".to_vec(),
                valid_principals: vec![principal.into_bytes()],
                valid_after,
                valid_before,
                critical_options: Vec::new(),
                extensions,
                reserved: Vec::new(),
                signature_key,
            },
        )
        .prepare()
        .map_err(|error| ApplicationError::Certificate(error.into()))
    }
}
