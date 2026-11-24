use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use keyringd_fido::webauthn::LargeBlobOutput;
use keyringd_fido::webauthn::{
    AuthenticationExtensionsClientOutputs, AuthenticatorAttachment,
    AuthenticatorAttestationResponse, PublicKeyCredential, PublicKeyCredentialType,
};

/// WebAuthn registration result and local device ID for specifying the same USB key for subsequent operations.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MakeCredentialResult {
    usb_device_id: Option<crate::usb::UsbHidDeviceId>,
    credential: PublicKeyCredential<AuthenticatorAttestationResponse>,
}

impl MakeCredentialResult {
    pub(crate) fn with_large_blob(mut self, output: LargeBlobOutput) -> Self {
        let extensions = self
            .credential
            .client_extension_results()
            .clone()
            .with_large_blob(output);
        self.credential = self.credential.with_client_extension_results(extensions);
        self
    }
    pub fn new(
        raw_id: Vec<u8>,
        response: AuthenticatorAttestationResponse,
        authenticator_attachment: Option<AuthenticatorAttachment>,
        client_extension_results: AuthenticationExtensionsClientOutputs,
    ) -> Self {
        Self {
            usb_device_id: None,
            credential: PublicKeyCredential::new(
                URL_SAFE_NO_PAD.encode(&raw_id),
                PublicKeyCredentialType::PublicKey,
                raw_id,
                response,
                authenticator_attachment,
                client_extension_results,
            ),
        }
    }

    /// A local identification value for specifying the same connected USB key for subsequent operations.
    pub fn usb_device_id(&self) -> Option<&crate::usb::UsbHidDeviceId> {
        self.usb_device_id.as_ref()
    }

    pub(crate) fn with_usb_device_id(mut self, id: Option<crate::usb::UsbHidDeviceId>) -> Self {
        self.usb_device_id = id;
        self
    }

    pub fn credential(&self) -> &PublicKeyCredential<AuthenticatorAttestationResponse> {
        &self.credential
    }
}
