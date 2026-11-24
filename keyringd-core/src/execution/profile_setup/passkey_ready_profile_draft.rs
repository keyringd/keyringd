use keyringd_ssh_agent_protocol::SshSignatureAlgorithm;

use crate::model::{
    ProfileId,
    device_key::{DeviceKey, LocalAuthentication},
    passkey::Passkey,
};

use super::DeviceKeyReadyProfileDraft;

#[derive(Clone)]
pub(super) struct PasskeyReadyProfileDraft {
    id: ProfileId,
    display_name: String,
    device_key_local_authentication: LocalAuthentication,
    device_key_signature_algorithm: SshSignatureAlgorithm,
    passkey: Passkey,
}

impl PasskeyReadyProfileDraft {
    pub(super) fn new(
        id: ProfileId,
        display_name: String,
        device_key_local_authentication: LocalAuthentication,
        device_key_signature_algorithm: SshSignatureAlgorithm,
        passkey: Passkey,
    ) -> Self {
        Self {
            id,
            display_name,
            device_key_local_authentication,
            device_key_signature_algorithm,
            passkey,
        }
    }

    pub(super) fn display_name(&self) -> &str {
        &self.display_name
    }
    pub(super) fn device_key_local_authentication(&self) -> LocalAuthentication {
        self.device_key_local_authentication
    }
    pub(super) fn device_key_signature_algorithm(&self) -> SshSignatureAlgorithm {
        self.device_key_signature_algorithm.clone()
    }

    pub(super) fn passkey(&self) -> &Passkey {
        &self.passkey
    }

    pub(super) fn with_device_key(self, device_key: DeviceKey) -> DeviceKeyReadyProfileDraft {
        DeviceKeyReadyProfileDraft::new(self.id, self.display_name, self.passkey, device_key)
    }
}
