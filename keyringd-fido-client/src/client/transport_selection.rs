use keyringd_fido::webauthn::PublicKeyCredentialHint;

use super::FidoClientError;

/// Internal values that classify HINTS of OPTIONS into USB, HYBRID, and INTERNAL routes.
/// If the HINT is not fixed to one, then the answer from the dialog handler will be determined.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TransportSelection {
    Hybrid,
    Usb,
    Internal,
}

impl TransportSelection {
    pub(super) fn from_hints(
        hints: &[PublicKeyCredentialHint],
    ) -> Result<Option<Self>, FidoClientError> {
        let usb = hints.iter().any(|hint| hint.as_str() == "security-key");
        let hybrid = hints.iter().any(|hint| hint.as_str() == "hybrid");
        let internal = hints.iter().any(|hint| hint.as_str() == "client-device");
        match (usb, hybrid, internal) {
            (true, false, false) => Ok(Some(Self::Usb)),
            (false, true, false) => Ok(Some(Self::Hybrid)),
            (false, false, true) => Ok(Some(Self::Internal)),
            _ => Ok(None),
        }
    }
}
