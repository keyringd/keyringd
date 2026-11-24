#[derive(Debug, thiserror::Error)]
pub enum WebAuthnError {
    #[error("Windows WebAuthn operation failed: {0}")]
    Windows(#[from] windows::core::Error),

    #[error("WebAuthn returned a null result")]
    NullResult,

    #[error("WebAuthn returned malformed authenticator data")]
    InvalidAuthenticatorData(#[source] keyringd_fido::ProtocolError),

    #[error("WebAuthn returned malformed attestation data")]
    InvalidAttestation(#[source] keyringd_fido::ProtocolError),

    #[error("WebAuthn returned a malformed UTF-16 attestation format")]
    InvalidAttestationFormat(#[source] std::string::FromUtf16Error),

    #[error("WebAuthn returned an empty signature")]
    InvalidSignature,

    #[error("Windows WebAuthn selected a non-USB authenticator")]
    NonUsbAuthenticator,

    #[error("WebAuthn value is too large")]
    ValueTooLarge,
}

impl WebAuthnError {
    pub fn is_user_cancellation(&self) -> bool {
        const HRESULT_CANCELLED_BY_USER: i32 = 0x8007_04C7_u32 as i32;
        matches!(self, Self::Windows(error) if error.code().0 == HRESULT_CANCELLED_BY_USER)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows::core::HRESULT;

    keyringd_testkit::scenario! {
        group "WebAuthnError::is_user_cancellation" {
            case "Identify Windows WebAuthn User Cancellations" {
                verify "Identify Windows WebAuthn User Cancellations" {
                    let error = WebAuthnError::Windows(windows::core::Error::from_hresult(HRESULT(
                        0x8007_04C7_u32 as i32,
                    )));
                    assert!(error.is_user_cancellation());
                }
            }
        }
    }
}
