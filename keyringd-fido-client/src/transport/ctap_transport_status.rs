/// Progress notification during CTAP communication. Indicates whether the process is being processed or waiting for the user's approval operation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
// Windows uses the platform WebAuthn backend; these CTAPHID keepalive values are
// constructed only by the native USB backend enabled on Linux and macOS.
#[cfg_attr(target_os = "windows", allow(dead_code))]
pub enum CtapTransportStatus {
    Processing,
    UserPresenceNeeded,
}
