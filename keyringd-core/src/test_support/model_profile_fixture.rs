use crate::model::{
    Profile, ProfileId,
    device_key::{DeviceKey, DevicePrivateKeyHandle, DevicePublicKey, LocalAuthentication},
    passkey::{CredentialId, Passkey},
};
use keyringd_ssh_agent_protocol::SshCertificateFields;
use keyringd_ssh_agent_protocol::SshSignatureAlgorithm;
use keyringd_ssh_agent_protocol::{
    EcdsaP256SshSignature, SshCertificateBody, SshCertificateType, SshSignature,
};

/// Direct model data only: no application, service, persistence, or clock access.
pub(crate) struct ModelProfileFixture {
    pub format: SshSignatureAlgorithm,
    pub authentication: LocalAuthentication,
    pub key_seed: u8,
    pub valid_after: u64,
    pub valid_before: u64,
}

impl Default for ModelProfileFixture {
    fn default() -> Self {
        Self {
            format: SshSignatureAlgorithm::EcdsaSha2NistP256,
            authentication: LocalAuthentication::NotRequired,
            key_seed: 11,
            valid_after: 100,
            valid_before: 200,
        }
    }
}

impl ModelProfileFixture {
    pub(crate) fn public_key(seed: u8) -> Vec<u8> {
        p256::ecdsa::SigningKey::from_slice(&[seed; 32])
            .unwrap()
            .verifying_key()
            .to_sec1_point(false)
            .as_bytes()
            .to_vec()
    }

    pub(crate) fn passkey(format: SshSignatureAlgorithm) -> Passkey {
        Passkey::new(
            format.clone(),
            CredentialId::new(vec![1, 2]),
            vec![7; 32],
            Self::public_key(7),
        )
        .unwrap()
    }

    pub(crate) fn build(&self) -> Profile {
        let passkey = Self::passkey(SshSignatureAlgorithm::SkEcdsaSha2NistP256);
        let public_key = DevicePublicKey::from_ecdsa_p256(&Self::public_key(self.key_seed))
            .unwrap()
            .to_ssh_public_key(
                (self.format != SshSignatureAlgorithm::EcdsaSha2NistP256).then_some(Passkey::RP_ID),
            )
            .unwrap();
        let certificate = SshCertificateBody::from_public_key(
            vec![1; 32],
            public_key,
            SshCertificateFields {
                serial: 1,
                certificate_type: SshCertificateType::User,
                key_id: b"model-test".to_vec(),
                valid_principals: vec![b"principal".to_vec()],
                valid_after: self.valid_after,
                valid_before: self.valid_before,
                critical_options: vec![],
                extensions: vec![],
                reserved: vec![],
                signature_key: passkey.to_ssh_public_key(),
            },
        )
        .prepare()
        .unwrap()
        // Signature validity is outside these model tests.
        .with_signature(SshSignature::Ecdsa(EcdsaP256SshSignature::new(
            [1; 32], [2; 32],
        )));
        Profile::new(
            ProfileId::from_stored("model-profile".into()),
            "Work".into(),
            true,
            DeviceKey::new(
                DevicePrivateKeyHandle::new(vec![3]),
                certificate,
                self.authentication,
                self.format.clone(),
            )
            .unwrap(),
            passkey,
        )
        .unwrap()
    }
}
