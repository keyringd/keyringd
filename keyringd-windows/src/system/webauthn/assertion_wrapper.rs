use std::ptr::NonNull;

use windows::Win32::Networking::WindowsWebServices::{WEBAUTHN_ASSERTION, WebAuthNFreeAssertion};

use super::WebAuthnError;

pub(super) struct AssertionWrapper {
    pub(super) pointer: NonNull<WEBAUTHN_ASSERTION>,
}

impl AssertionWrapper {
    pub(super) fn new(pointer: *mut WEBAUTHN_ASSERTION) -> Result<Self, WebAuthnError> {
        Ok(Self {
            pointer: NonNull::new(pointer).ok_or(WebAuthnError::NullResult)?,
        })
    }
}

impl Drop for AssertionWrapper {
    fn drop(&mut self) {
        unsafe { WebAuthNFreeAssertion(self.pointer.as_ptr()) };
    }
}
