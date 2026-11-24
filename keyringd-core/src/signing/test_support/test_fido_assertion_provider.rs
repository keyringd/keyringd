use crate::application::ApplicationError;
use crate::signing::fido::FidoAssertionProvider;
use keyringd_fido::webauthn::AuthenticatorData;
use keyringd_fido_client::{GetAssertionResult, GetAssertionsResult};
use sha2::{Digest, Sha256};

pub(crate) struct TestFidoAssertionProvider {
    authenticator_data: AuthenticatorData,
}

impl TestFidoAssertionProvider {
    pub fn new(application: &str, extensions: &[u8]) -> Self {
        let mut raw = Sha256::digest(application.as_bytes()).to_vec();
        raw.push(if extensions.is_empty() { 5 } else { 0x85 });
        raw.extend_from_slice(&42u32.to_be_bytes());
        raw.extend_from_slice(extensions);
        Self {
            authenticator_data: AuthenticatorData::from_bytes(&raw).unwrap(),
        }
    }
}

impl FidoAssertionProvider for TestFidoAssertionProvider {
    async fn get_assertion(
        &mut self,
        options: keyringd_fido::webauthn::PublicKeyCredentialRequestOptions,
        client_data_provider: std::sync::Arc<dyn keyringd_fido_client::ClientDataProvider>,
    ) -> Result<GetAssertionResult, ApplicationError> {
        use p256::ecdsa::{Signature, SigningKey, signature::Signer};
        let client_data = client_data_provider
            .provide(options.challenge())
            .map_err(|error| ApplicationError::InvalidAuthenticatorResponse(error.into()))?;
        let mut signed_data = self.authenticator_data.raw().to_vec();
        signed_data.extend_from_slice(&Sha256::digest(client_data.as_bytes()));
        let signing_key = SigningKey::from_slice(&[1; 32]).unwrap();
        let signature: Signature = signing_key.sign(&signed_data);
        let response = keyringd_fido::ctap::AuthenticatorGetAssertionResponse::new(
            Some(keyringd_fido::webauthn::PublicKeyCredentialDescriptor::new(
                keyringd_fido::webauthn::PublicKeyCredentialType::PublicKey,
                vec![1],
            )),
            self.authenticator_data.clone(),
            signature.to_der().as_bytes().to_vec(),
        );
        GetAssertionsResult::from_ctap(
            vec![response],
            client_data,
            keyringd_fido::AuthenticatorTransport::usb(),
            None,
        )
        .unwrap()
        .select(None)
        .map_err(ApplicationError::from)
    }
}
