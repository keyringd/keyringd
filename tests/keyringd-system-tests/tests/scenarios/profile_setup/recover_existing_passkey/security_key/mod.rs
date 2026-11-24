mod cancellation;
mod configured_pin;
// Windows WebAuthn returns one OS-selected credential; app-side candidate
// selection is exercised over real Hybrid transport on every OS instead.
#[cfg(unix)]
mod multiple_credentials;
mod multiple_keys;
mod passkey_not_found;
mod success;
mod user_verification;
