use std::ptr::NonNull;

use windows::Win32::Networking::WindowsWebServices::{
    WEBAUTHN_CREDENTIAL_ATTESTATION, WebAuthNFreeCredentialAttestation,
};

use super::WebAuthnError;

pub(super) struct CredentialAttestation {
    pub(super) pointer: NonNull<WEBAUTHN_CREDENTIAL_ATTESTATION>,
}

impl CredentialAttestation {
    pub(super) fn new(
        pointer: *mut WEBAUTHN_CREDENTIAL_ATTESTATION,
    ) -> Result<Self, WebAuthnError> {
        Ok(Self {
            pointer: NonNull::new(pointer).ok_or(WebAuthnError::NullResult)?,
        })
    }
}

impl Drop for CredentialAttestation {
    fn drop(&mut self) {
        unsafe { WebAuthNFreeCredentialAttestation(Some(self.pointer.as_ptr())) };
    }
}
