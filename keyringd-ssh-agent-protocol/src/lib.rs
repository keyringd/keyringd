extern crate self as keyringd_ssh_agent_protocol;
mod connection;
pub mod extension_names;
mod protocol;
mod ssh_agent;
mod ssh_agent_config;
mod ssh_agent_listener;

pub use connection::{SshAgentConnection, SshAgentRequestHandler};
pub use protocol::{
    EcdsaP256CertificateBody, EcdsaP256PublicKey, EcdsaP256SshSignature,
    SkEcdsaP256CertificateBody, SkEcdsaP256PublicKey, SkEcdsaP256SshSignature, SshAgentCodecError,
    SshAgentExtensionRequest, SshAgentIdentity, SshAgentRequest, SshAgentResponse,
    SshAgentSignFlags, SshAgentSignRequest, SshCertificate, SshCertificateBody,
    SshCertificateCriticalOption, SshCertificateExtension, SshCertificateFields,
    SshCertificateSigningRequest, SshCertificateType, SshEllipticCurve, SshKeyAlgorithm,
    SshPublicKey, SshPublicKeyBlob, SshSignature, SshSignatureAlgorithm, SshSigningPayload,
    SshSigningPayloadError, SshUserauthSigningPayload, SshsigSigningPayload,
    WebAuthnEcdsaP256SshSignature,
};
pub use ssh_agent::SshAgent;
pub use ssh_agent_config::SshAgentConfig;
pub use ssh_agent_listener::SshAgentListener;

#[cfg(test)]
mod test_support;

#[cfg(test)]
mod tests;
