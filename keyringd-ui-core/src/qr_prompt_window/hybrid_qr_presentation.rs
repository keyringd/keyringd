use super::prompt;
use crate::key_operation::AuthenticatorPromptPurpose;
use keyringd_fido_client::HybridTransportUnavailableReason;

/// QR content shared by standalone prompts and embedded setup interactions.
#[derive(Clone)]
pub struct HybridQrPresentation {
    pub value: String,
    context: AuthenticatorPromptPurpose,
    pub security_key_available: bool,
    pub unavailable: Option<HybridTransportUnavailableReason>,
}

impl HybridQrPresentation {
    pub fn new(
        value: impl Into<String>,
        context: &AuthenticatorPromptPurpose,
        security_key_available: bool,
        unavailable: Option<HybridTransportUnavailableReason>,
    ) -> Self {
        Self {
            value: value.into(),
            context: context.clone(),
            security_key_available: security_key_available && context.allows_security_key_choice(),
            unavailable,
        }
    }

    pub fn status(&self) -> &'static str {
        crate::qr_prompt_window::hybrid_unavailable_message(self.unavailable)
    }

    pub fn title(&self) -> &'static str {
        prompt::heading(&self.context)
    }
    pub fn instruction(&self) -> &'static str {
        prompt::instruction(&self.context)
    }
    pub fn explanation(&self) -> String {
        prompt::explanation(&self.context)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    keyringd_testkit::scenario! {
        group "HybridQrPresentation::instruction" {
            case "Bound recovery cannot offer another authenticator" {
                verify "hide the security-key switch when recovery is bound to its chosen transport" {
                    let presentation = HybridQrPresentation::new(
                        "FIDO:/verify",
                        &AuthenticatorPromptPurpose::RecoverPasskeyPublicKey,
                        true,
                        None,
                    );
                    assert!(!presentation.security_key_available);
                    assert!(presentation.instruction().contains("again with the same"));
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "HybridQrPresentation::status" {
            case "Bluetooth failure keeps the qr and available usb alternative" {
                verify "show the Hybrid error on the QR screen and keep the USB option actionable" {
                    let presentation = HybridQrPresentation::new(
                        "FIDO:/find",
                        &AuthenticatorPromptPurpose::FindExistingPasskey,
                        true,
                        Some(HybridTransportUnavailableReason::BluetoothDisabled),
                    );
                    assert_eq!(presentation.value, "FIDO:/find");
                    assert!(presentation.security_key_available);
                    assert!(presentation.status().contains("Bluetooth is off"));
                }
            }
        }
    }
}
