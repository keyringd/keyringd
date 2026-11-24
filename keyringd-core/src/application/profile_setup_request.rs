use super::PasskeyChoice;
use keyringd_ssh_agent_protocol::SshSignatureAlgorithm;

pub struct ProfileSetupRequest {
    pub display_name: String,
    pub passkey: PasskeyChoice,
    pub device_key_signature_algorithm: SshSignatureAlgorithm,
    pub passkey_signature_algorithm: SshSignatureAlgorithm,
}

impl ProfileSetupRequest {
    pub fn new(display_name: String, passkey: PasskeyChoice) -> Self {
        Self {
            display_name,
            passkey,
            device_key_signature_algorithm: SshSignatureAlgorithm::EcdsaSha2NistP256,
            passkey_signature_algorithm: SshSignatureAlgorithm::SkEcdsaSha2NistP256,
        }
    }
}
