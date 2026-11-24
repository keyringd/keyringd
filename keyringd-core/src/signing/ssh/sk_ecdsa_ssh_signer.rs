use super::SshSigner;
use crate::application::ApplicationError;
use crate::model::EcdsaP256Signature;
use crate::signing::fido::FidoAssertionProvider;
use keyringd_fido_client::ClientData;
use keyringd_ssh_agent_protocol::{SkEcdsaP256SshSignature, SshSignature};
use std::sync::Arc;

pub(crate) struct SkEcdsaSshSigner<S> {
    provider: S,
}

impl<S: FidoAssertionProvider> SkEcdsaSshSigner<S> {
    pub fn new(provider: S) -> Self {
        Self { provider }
    }

    pub(crate) async fn sign_with_options(
        &mut self,
        options: keyringd_fido::webauthn::PublicKeyCredentialRequestOptions,
    ) -> Result<SshSignature, ApplicationError> {
        let message = options.challenge().to_vec();
        let assertion = self
            .provider
            .get_assertion(
                options,
                Arc::new(|challenge: &[u8]| Ok(ClientData::from_bytes(challenge.to_vec()))),
            )
            .await?;
        let response = assertion.response();
        if response
            .authenticator_data()
            .attested_credential_data()
            .is_some()
            || response.authenticator_data().extensions().is_some()
        {
            return Err(ApplicationError::InvalidAuthenticatorResponse(
                "SK SSH signatures cannot encode attested credential data or authenticator extensions".into(),
            ));
        }
        if assertion.client_data().as_bytes() != message {
            return Err(ApplicationError::InvalidAuthenticatorResponse(
                "security-key client data does not match the SSH message".into(),
            ));
        }
        Ok(SkEcdsaP256SshSignature::new(
            EcdsaP256Signature::from_der(response.signature())
                .map_err(|error| ApplicationError::InvalidAuthenticatorResponse(error.into()))?
                .to_ssh_signature(),
            response.authenticator_data().flags(),
            response.authenticator_data().sign_count(),
        )
        .into_signature())
    }
}

impl<S: FidoAssertionProvider> SshSigner for SkEcdsaSshSigner<S> {
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

    keyringd_testkit::scenario! {
        group "SkEcdsaSshSigner::sign" {
            case "Rejects signed extensions that the format cannot transmit" {
                async: tokio;
                timeout: std::time::Duration::from_secs(10);
                verify "fail signing when authenticator extensions cannot travel in the SSH signature" {
                    let mut signer =
                        SkEcdsaSshSigner::new(TestFidoAssertionProvider::new("ssh:keyringd", &[0xa0]));
                    let error = signer.sign(b"SSH message").await.unwrap_err();
                    assert!(error.to_string().contains("cannot encode"));
                }
            }

            case "Leaves rp id hash verification to the server" {
                async: tokio;
                timeout: std::time::Duration::from_secs(10);
                verify "return the authenticator signature for verification by the relying party" {
                    let mut signer = SkEcdsaSshSigner::new(TestFidoAssertionProvider::new("ssh:different", &[]));
                    let signature = signer.sign(b"SSH message").await.unwrap();
                    assert_eq!(
                        signature.algorithm(),
                        SshSignatureAlgorithm::SkEcdsaSha2NistP256
                    );
                    let SshSignature::SkEcdsa(signature) = signature else {
                        panic!("expected SK signature");
                    };
                    assert_eq!(signature.flags(), 5);
                    assert_eq!(signature.counter(), 42);
                }
            }
        }
    }
}
