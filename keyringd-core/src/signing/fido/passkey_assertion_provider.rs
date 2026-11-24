use super::{FidoAssertionProvider, passkey_assertion_options};
use crate::execution::operation::FidoOperationTarget;
use crate::model::passkey::{Passkey, PasskeyAttestationResponse};
use crate::signing::ssh::{SkEcdsaSshSigner, WebAuthnSkEcdsaSshSigner};
use crate::{application::ApplicationError, service::PasskeyService};
use keyringd_fido::webauthn::LargeBlobOutput;
use keyringd_fido::webauthn::PublicKeyCredentialRequestOptions;
use keyringd_fido::webauthn::{
    AuthenticationExtensionsClientInputs, LargeBlobInput, PublicKeyCredentialDescriptor,
    PublicKeyCredentialType,
};
use keyringd_fido_client::{ClientDataProvider, GetAssertionResult, GetAssertionsResult};
use keyringd_ssh_agent_protocol::{SshSignature, SshSignatureAlgorithm};
use std::sync::Arc;

/// Forwards assertion options and retains authenticator selection across operations.
/// Generic client-data collection and response validation belong to the FIDO library.
pub(crate) struct PasskeyAssertionProvider<'a, F> {
    service: &'a F,
    target: &'a mut FidoOperationTarget,
}
impl<'a, F: PasskeyService> PasskeyAssertionProvider<'a, F> {
    pub(crate) fn new(service: &'a F, target: &'a mut FidoOperationTarget) -> Self {
        Self { service, target }
    }
    pub(crate) async fn get_assertions(
        &mut self,
        options: PublicKeyCredentialRequestOptions,
        provider: Arc<dyn ClientDataProvider>,
    ) -> Result<GetAssertionsResult, ApplicationError> {
        let writing = options
            .extensions()
            .and_then(|e| e.large_blob())
            .is_some_and(|input| input.write().is_some());
        let result = self
            .service
            .get_assertions(options, self.target.usb_device_id().cloned(), provider)
            .await?;
        if writing
            && !result
                .responses()
                .iter()
                .all(|r| matches!(r.large_blob(), Some(LargeBlobOutput::Written(true))))
        {
            tracing::warn!(
                "Passkey registration evidence was not saved to largeBlob; keeping the local copy"
            );
        }
        self.target
            .remember(result.transport(), result.usb_device_id())?;
        Ok(result)
    }

    pub(crate) async fn sign_ssh(
        &mut self,
        passkey: &Passkey,
        message: &[u8],
    ) -> Result<SshSignature, ApplicationError> {
        self.sign_ssh_with_blob(passkey, message, None).await
    }

    pub(crate) async fn sign_ssh_and_try_write_attestation(
        &mut self,
        passkey: &Passkey,
        message: &[u8],
    ) -> Result<SshSignature, ApplicationError> {
        let data = passkey
            .attestation_response()
            .map(PasskeyAttestationResponse::to_large_blob)
            .transpose()
            .map_err(|error| ApplicationError::InvariantViolation(error.into()))?;
        self.sign_ssh_with_blob(passkey, message, data).await
    }

    async fn sign_ssh_with_blob(
        &mut self,
        passkey: &Passkey,
        message: &[u8],
        large_blob_write: Option<Vec<u8>>,
    ) -> Result<SshSignature, ApplicationError> {
        let credential = PublicKeyCredentialDescriptor::new(
            PublicKeyCredentialType::PublicKey,
            passkey.credential_id().as_bytes().to_vec(),
        );
        let mut options = passkey_assertion_options(message, &self.target.hints())
            .with_allow_credentials(vec![credential]);
        if let Some(data) = large_blob_write {
            options = options.with_extensions(
                AuthenticationExtensionsClientInputs::default()
                    .with_large_blob(LargeBlobInput::request_write(data)),
            );
        }
        match passkey.signature_algorithm() {
            SshSignatureAlgorithm::SkEcdsaSha2NistP256 => {
                SkEcdsaSshSigner::new(self).sign_with_options(options).await
            }
            SshSignatureAlgorithm::WebAuthnSkEcdsaSha2NistP256 => {
                WebAuthnSkEcdsaSshSigner::new(self, Passkey::WEB_AUTHN_ORIGIN.to_owned())
                    .sign_with_options(options)
                    .await
            }
            SshSignatureAlgorithm::EcdsaSha2NistP256 | SshSignatureAlgorithm::Unsupported(_) => {
                unreachable!("Passkey guarantees an SK-ECDSA or WebAuthn algorithm")
            }
        }
    }
}

impl<F: PasskeyService> FidoAssertionProvider for &mut PasskeyAssertionProvider<'_, F> {
    async fn get_assertion(
        &mut self,
        options: PublicKeyCredentialRequestOptions,
        provider: Arc<dyn ClientDataProvider>,
    ) -> Result<GetAssertionResult, ApplicationError> {
        let [credential] = options.allow_credentials() else {
            return Err(ApplicationError::InvariantViolation(
                "SSH passkey signing requires exactly one selected credential".into(),
            ));
        };
        let credential_id = credential.id().to_vec();
        let result = self.get_assertions(options, provider).await?;
        result
            .select(Some(&credential_id))
            .map_err(ApplicationError::from)
    }
}
