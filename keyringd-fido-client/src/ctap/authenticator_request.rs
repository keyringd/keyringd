use keyringd_fido::ctap::{AuthenticatorGetAssertionRequest, AuthenticatorMakeCredentialRequest};

/// A reference to a CTAP request for validation. Pass the request that the operation context has without duplicating it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AuthenticatorRequest<'a> {
    MakeCredential(&'a AuthenticatorMakeCredentialRequest),
    GetAssertion(&'a AuthenticatorGetAssertionRequest),
}
