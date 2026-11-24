use serde::{Deserialize, Serialize};
#[derive(Deserialize, Serialize)]
pub enum SecurityAutomationAction {
    IsElevated,
    WaitOpen,
    WaitClosed { window_handle: usize },
    Capture,
    WaitEditors { count: i32 },
    CreatePassword { value: String },
    EnterPassword { value: String },
    WaitPasswordRejected,
    Dismiss,
    WaitWebAuthnOpen,
    WaitWebAuthnClosed,
    WebAuthnEditors,
    CaptureWebAuthn,
    EnterWebAuthnPin { value: String },
    Shutdown,
}
