use crate::key_operation::AuthenticatorPromptPurpose;
use crate::profile_setup_window::ProfileSetupStep;
use crate::text;

pub fn heading(purpose: &AuthenticatorPromptPurpose) -> &'static str {
    if let Some(step) = ProfileSetupStep::for_purpose(purpose) {
        return step.heading();
    }
    match purpose {
        AuthenticatorPromptPurpose::DeviceKeyProtectionChange { .. } => {
            text::QR_PROMPT_PROTECTION_HEADING
        }
        AuthenticatorPromptPurpose::DeviceKeyReplacement { .. } => {
            text::QR_PROMPT_REPLACEMENT_HEADING
        }
        AuthenticatorPromptPurpose::AgentRequest => text::QR_PROMPT_AGENT_HEADING,
        _ => text::QR_PROMPT_DEFAULT_HEADING,
    }
}

pub fn description(purpose: &AuthenticatorPromptPurpose) -> String {
    if let Some(step) = ProfileSetupStep::for_purpose(purpose) {
        return step.description().into();
    }
    match purpose {
        AuthenticatorPromptPurpose::DeviceKeyProtectionChange { profile_name } => format!(
            "Scan this QR code with the phone or tablet that contains the Passkey for “{profile_name}”. The Passkey must authorize the replacement Device Key before it can be used."
        ),
        AuthenticatorPromptPurpose::DeviceKeyReplacement { profile_name } => format!(
            "Scan this QR code with the phone or tablet that contains the Passkey for “{profile_name}”."
        ),
        AuthenticatorPromptPurpose::AgentRequest => text::QR_PROMPT_AGENT_DESCRIPTION.into(),
        _ => text::QR_PROMPT_DEFAULT_DESCRIPTION.into(),
    }
}

pub fn instruction(purpose: &AuthenticatorPromptPurpose) -> &'static str {
    ProfileSetupStep::for_purpose(purpose).map_or("", ProfileSetupStep::scan_instruction)
}

pub fn explanation(purpose: &AuthenticatorPromptPurpose) -> String {
    let description = description(purpose);
    let instruction = instruction(purpose);
    if instruction.is_empty() {
        description
    } else {
        format!("{description}\n\n{instruction}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    keyringd_testkit::scenario! {
        group "instruction" {
            case "Recovery prompts distinguish the first scan from verification and authorization" {
                verify "render distinct QR instructions for discovery, verification, and authorization" {
                    assert!(!instruction(&AuthenticatorPromptPurpose::FindExistingPasskey).contains("again"));
                    assert!(
                        instruction(&AuthenticatorPromptPurpose::RecoverPasskeyPublicKey)
                            .contains("again with the same")
                    );
                    assert!(
                        instruction(&AuthenticatorPromptPurpose::AuthorizeWithExistingPasskey)
                            .contains("one last time")
                    );
                }
            }
        }
    }
}
