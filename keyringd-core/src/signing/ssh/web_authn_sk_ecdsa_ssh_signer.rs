use super::SshSigner;
use crate::application::ApplicationError;
use crate::model::EcdsaP256Signature;
use crate::signing::fido::FidoAssertionProvider;
use keyringd_fido::CollectedClientData;
use keyringd_fido_client::{ClientData, ClientDataProviderError};
use keyringd_ssh_agent_protocol::{SshSignature, WebAuthnEcdsaP256SshSignature};
use std::sync::Arc;

// RP ID hash (32 bytes), flags (1 byte), and signature counter (4 bytes).
const AUTHENTICATOR_DATA_HEADER_LEN: usize = 32 + 1 + 4;

pub(crate) struct WebAuthnSkEcdsaSshSigner<S> {
    provider: S,
    origin: String,
}

impl<S: FidoAssertionProvider> WebAuthnSkEcdsaSshSigner<S> {
    pub fn new(provider: S, origin: String) -> Self {
        Self { provider, origin }
    }

    pub(crate) async fn sign_with_options(
        &mut self,
        options: keyringd_fido::webauthn::PublicKeyCredentialRequestOptions,
    ) -> Result<SshSignature, ApplicationError> {
        let message = options.challenge().to_vec();
        let origin = self.origin.clone();
        let assertion = self
            .provider
            .get_assertion(
                options,
                Arc::new(move |challenge: &[u8]| {
                    CollectedClientData::get(challenge, &origin)
                        .and_then(ClientData::try_from)
                        .map_err(ClientDataProviderError::new)
                }),
            )
            .await?;
        let response = assertion.response();
        if response
            .authenticator_data()
            .attested_credential_data()
            .is_some()
        {
            return Err(ApplicationError::InvalidAuthenticatorResponse(
                "WebAuthn SSH signatures cannot encode attested credential data".into(),
            ));
        }
        let collected = CollectedClientData::from_json_bytes(assertion.client_data().as_bytes())
            .map_err(|error| ApplicationError::InvalidAuthenticatorResponse(error.into()))?;
        let challenge = collected
            .challenge_bytes()
            .map_err(|error| ApplicationError::InvalidAuthenticatorResponse(error.into()))?;
        if collected.ceremony_type() != "webauthn.get"
            || challenge != message
            || collected.cross_origin() == Some(true)
        {
            return Err(ApplicationError::InvalidAuthenticatorResponse(
                "WebAuthn client data does not match the SSH message".into(),
            ));
        }
        Ok(WebAuthnEcdsaP256SshSignature::new(
            EcdsaP256Signature::from_der(response.signature())
                .map_err(|error| ApplicationError::InvalidAuthenticatorResponse(error.into()))?
                .to_ssh_signature(),
            response.authenticator_data().flags(),
            response.authenticator_data().sign_count(),
            collected.origin().to_string(),
            assertion.client_data().as_bytes().to_vec(),
            // Preserve the signed CBOR bytes rather than re-encoding extension values.
            response.authenticator_data().raw()[AUTHENTICATOR_DATA_HEADER_LEN..].to_vec(),
        )
        .into_signature())
    }
}

impl<S: FidoAssertionProvider> SshSigner for WebAuthnSkEcdsaSshSigner<S> {
    async fn sign(&mut self, message: &[u8]) -> Result<SshSignature, ApplicationError> {
        self.sign_with_options(
            keyringd_fido::webauthn::PublicKeyCredentialRequestOptions::new(message.to_vec()),
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::signing::test_support::TestFidoAssertionProvider;
    use keyringd_ssh_agent_protocol::SshSignatureAlgorithm;
    use p256::ecdsa::{Signature, SigningKey, signature::Verifier};
    use sha2::{Digest, Sha256};

    fn scalar(mpint: &[u8]) -> [u8; 32] {
        let mpint = mpint.strip_prefix(&[0]).unwrap_or(mpint);
        let mut scalar = [0; 32];
        scalar[32 - mpint.len()..].copy_from_slice(mpint);
        scalar
    }

    keyringd_testkit::scenario! {
        group "WebAuthnSkEcdsaSshSigner::sign" {
            case "Transmits exact extensions and retains a verifiable signature" {
                async: tokio;
                timeout: std::time::Duration::from_secs(10);
                verify "preserve the authenticator extension bytes and verify the resulting signature" {
                    for extensions in [&[][..], &[0xa1, 0x61, b'x', 0xf5][..]] {
                        let mut signer = WebAuthnSkEcdsaSshSigner::new(TestFidoAssertionProvider::new("ssh:keyringd", extensions), "https://keyringd.example".into());
                        let message = b"SSH message";
                        let signature = signer.sign(message).await.unwrap();
                        assert_eq!(
                            signature.algorithm(),
                            SshSignatureAlgorithm::WebAuthnSkEcdsaSha2NistP256
                        );
                        let SshSignature::WebAuthnSkEcdsa(signature) = signature else {
                            panic!("expected WebAuthn signature");
                        };
                        let r = scalar(signature.signature().r());
                        let s = scalar(signature.signature().s());
                        assert_eq!(signature.origin(), "https://keyringd.example");
                        let client_data = signature.client_data();
                        let transmitted_extensions = signature.extensions();
                        assert_eq!(transmitted_extensions, extensions);
                        let mut signed_data = Sha256::digest(b"ssh:keyringd").to_vec();
                        signed_data.push(signature.flags());
                        signed_data.extend_from_slice(&signature.counter().to_be_bytes());
                        signed_data.extend_from_slice(transmitted_extensions);
                        signed_data.extend_from_slice(&Sha256::digest(client_data));
                        let key = SigningKey::from_slice(&[1; 32]).unwrap();
                        key.verifying_key()
                            .verify(&signed_data, &Signature::from_scalars(r, s).unwrap())
                            .unwrap();
                    }
                }
            }
        }
    }
}
