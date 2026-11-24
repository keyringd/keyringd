use windows::Win32::Networking::WindowsWebServices::WEBAUTHN_AUTHENTICATOR_GET_ASSERTION_OPTIONS;
use windows::core::PCWSTR;

#[repr(C)]
pub(super) struct GetAssertionOptionsV8 {
    pub base: WEBAUTHN_AUTHENTICATOR_GET_ASSERTION_OPTIONS,
    pub c_credential_hints: u32,
    pub credential_hints: *mut PCWSTR,
}
