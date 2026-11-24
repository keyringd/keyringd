use keyringd_ssh_agent_protocol::SshSignatureAlgorithm;

use crate::model::{ProfileId, device_key::LocalAuthentication, passkey::Passkey};

use super::PasskeyReadyProfileDraft;

#[derive(Clone)]
pub(super) struct ProfileDraft {
    id: ProfileId,
    display_name: String,
    device_key_local_authentication: LocalAuthentication,
    passkey_signature_algorithm: SshSignatureAlgorithm,
    device_key_signature_algorithm: SshSignatureAlgorithm,
}

impl ProfileDraft {
    pub(super) fn new(
        id: ProfileId,
        display_name: String,
        device_key_local_authentication: LocalAuthentication,
        passkey_signature_algorithm: SshSignatureAlgorithm,
        device_key_signature_algorithm: SshSignatureAlgorithm,
    ) -> Self {
        Self {
            id,
            display_name,
            device_key_local_authentication,
            passkey_signature_algorithm,
            device_key_signature_algorithm,
        }
    }

    pub(super) fn passkey_signature_algorithm(&self) -> SshSignatureAlgorithm {
        self.passkey_signature_algorithm.clone()
    }

    pub(super) fn with_passkey(self, passkey: Passkey) -> PasskeyReadyProfileDraft {
        PasskeyReadyProfileDraft::new(
            self.id,
            self.display_name,
            self.device_key_local_authentication,
            self.device_key_signature_algorithm,
            passkey,
        )
    }
}
