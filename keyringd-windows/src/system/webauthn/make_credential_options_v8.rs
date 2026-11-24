use windows::Win32::Networking::WindowsWebServices::{
    WEBAUTHN_AUTHENTICATOR_MAKE_CREDENTIAL_OPTIONS, WEBAUTHN_HMAC_SECRET_SALT,
};
use windows::core::{BOOL, PCWSTR};

/// Fields added by WEBAUTHN_AUTHENTICATOR_MAKE_CREDENTIAL_OPTIONS_VERSION_8.
#[repr(C)]
pub(super) struct MakeCredentialOptionsV8 {
    pub base: WEBAUTHN_AUTHENTICATOR_MAKE_CREDENTIAL_OPTIONS,
    pub prf_global_eval: *mut WEBAUTHN_HMAC_SECRET_SALT,
    pub credential_hint_count: u32,
    pub credential_hints: *mut PCWSTR,
    pub third_party_payment: BOOL,
}
