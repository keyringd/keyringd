use crate::key_operation::AuthenticatorPromptKind;
use crate::key_operation::AuthenticatorPromptPurpose;
use crate::pin_prompt_window::SecurityKeyPinPrompt;
use crate::text;

pub fn operation_prompt_text(
    operation: &crate::key_operation::KeyOperationKind,
    prompt_kind: AuthenticatorPromptKind,
    retry: bool,
) -> (&'static str, String) {
    use crate::key_operation::KeyOperationKind;
    let title = match prompt_kind {
        AuthenticatorPromptKind::AuthenticatorSelection => text::SECURITY_KEY_SELECTION_HEADING,
        AuthenticatorPromptKind::BuiltInUserVerification => text::SECURITY_KEY_VERIFICATION_HEADING,
        AuthenticatorPromptKind::UserPresence if retry => text::SECURITY_KEY_RETRY_HEADING,
        AuthenticatorPromptKind::UserPresence => text::SECURITY_KEY_APPROVAL_HEADING,
    };
    if prompt_kind == AuthenticatorPromptKind::AuthenticatorSelection {
        return (title, text::SECURITY_KEY_SELECTION_DESCRIPTION.into());
    }
    let action = match operation {
        KeyOperationKind::DeviceKeyProtectionChange { profile_name } => {
            format!("authorize the Local authentication change for “{profile_name}”")
        }
        KeyOperationKind::DeviceKeyReplacement { profile_name } => {
            format!("authorize the new Device Key for “{profile_name}”")
        }
        KeyOperationKind::ProfileSetup { .. } => text::SECURITY_KEY_SETUP_ACTION.into(),
        KeyOperationKind::AgentRequest => text::SECURITY_KEY_SSH_ACTION.into(),
    };
    let explanation = if prompt_kind == AuthenticatorPromptKind::BuiltInUserVerification {
        format!(
            "Verify your identity on your security key to {action}. Use its built-in verification, such as its fingerprint sensor."
        )
    } else if retry {
        format!("The key did not respond in time. Touch it again to {action}.")
    } else {
        format!("Touch your security key to {action}.")
    };
    (title, explanation)
}

pub fn pin(
    request: &SecurityKeyPinPrompt,
    context: Option<&AuthenticatorPromptPurpose>,
) -> (&'static str, String) {
    let explanation = crate::pin_prompt_window::pin_explanation(
        &pin_instruction(request, context),
        request.minimum_length,
        request.attempts_left,
    );
    (
        crate::pin_prompt_window::pin_heading(request.new_pin),
        explanation,
    )
}

pub fn pin_instruction(
    request: &SecurityKeyPinPrompt,
    context: Option<&AuthenticatorPromptPurpose>,
) -> String {
    if request.new_pin {
        request.reason.clone()
    } else {
        match context {
            Some(AuthenticatorPromptPurpose::CreatePasskey) => text::SECURITY_KEY_CREATE_PIN,
            Some(AuthenticatorPromptPurpose::AuthorizeWithNewPasskey) => {
                text::SECURITY_KEY_AUTHORIZE_NEW_PIN
            }
            Some(AuthenticatorPromptPurpose::FindExistingPasskey) => text::SECURITY_KEY_FIND_PIN,
            Some(AuthenticatorPromptPurpose::RecoverPasskeyPublicKey) => {
                text::SECURITY_KEY_VERIFY_PIN
            }
            Some(AuthenticatorPromptPurpose::AuthorizeWithExistingPasskey) => {
                text::SECURITY_KEY_AUTHORIZE_EXISTING_PIN
            }
            _ => &request.reason,
        }
        .to_owned()
    }
}

pub fn waiting(
    transport: Option<&keyringd_fido::AuthenticatorTransport>,
    prompt_kind: Option<AuthenticatorPromptKind>,
    pin_requested: bool,
    context: Option<&AuthenticatorPromptPurpose>,
) -> (&'static str, String) {
    let (title, explanation) = match transport.map(|t| t.as_str()) {
        Some("usb") => match prompt_kind {
            Some(AuthenticatorPromptKind::BuiltInUserVerification) => (
                text::SECURITY_KEY_VERIFICATION_HEADING,
                security_key_uv_instruction(context),
            ),
            Some(AuthenticatorPromptKind::AuthenticatorSelection) => (
                text::SECURITY_KEY_SELECTION_HEADING,
                text::SECURITY_KEY_SELECTION_DESCRIPTION.to_owned(),
            ),
            Some(AuthenticatorPromptKind::UserPresence) => (
                approval_title(context),
                security_key_touch_instruction(context, pin_requested),
            ),
            None => (
                text::SECURITY_KEY_DEFAULT_HEADING,
                text::SECURITY_KEY_DEFAULT_DESCRIPTION.to_owned(),
            ),
        },
        Some("hybrid") => (
            text::AUTHENTICATOR_PHONE_HEADING,
            text::AUTHENTICATOR_PHONE_DESCRIPTION.to_owned(),
        ),
        Some(_) => (
            text::PROFILE_SETUP_CONFIRM_OPERATION,
            text::AUTHENTICATOR_DEFAULT_DESCRIPTION.to_owned(),
        ),
        None => (
            text::PROFILE_SETUP_PREPARING,
            text::PROFILE_SETUP_PREPARING_NEXT.to_owned(),
        ),
    };
    (title, explanation)
}

fn approval_title(context: Option<&AuthenticatorPromptPurpose>) -> &'static str {
    match context {
        Some(AuthenticatorPromptPurpose::CreatePasskey) => text::SECURITY_KEY_CREATE_HEADING,
        Some(
            AuthenticatorPromptPurpose::AuthorizeWithNewPasskey
            | AuthenticatorPromptPurpose::AuthorizeWithExistingPasskey,
        ) => text::SECURITY_KEY_AUTHORIZE_HEADING,
        Some(AuthenticatorPromptPurpose::FindExistingPasskey) => text::SECURITY_KEY_FIND_HEADING,
        Some(AuthenticatorPromptPurpose::RecoverPasskeyPublicKey) => {
            text::SECURITY_KEY_VERIFY_HEADING
        }
        _ => text::SECURITY_KEY_APPROVAL_HEADING,
    }
}
fn security_key_touch_instruction(
    context: Option<&AuthenticatorPromptPurpose>,
    pin_requested: bool,
) -> String {
    let instruction = match context {
        Some(AuthenticatorPromptPurpose::CreatePasskey) => text::SECURITY_KEY_CREATE_TOUCH,
        Some(AuthenticatorPromptPurpose::AuthorizeWithNewPasskey) => {
            text::SECURITY_KEY_AUTHORIZE_NEW_TOUCH
        }
        Some(AuthenticatorPromptPurpose::FindExistingPasskey) => text::SECURITY_KEY_FIND_TOUCH,
        Some(AuthenticatorPromptPurpose::RecoverPasskeyPublicKey) => {
            text::SECURITY_KEY_VERIFY_TOUCH
        }
        Some(AuthenticatorPromptPurpose::AuthorizeWithExistingPasskey) => {
            text::SECURITY_KEY_AUTHORIZE_EXISTING_TOUCH
        }
        _ => text::SECURITY_KEY_DEFAULT_TOUCH,
    };
    if pin_requested {
        format!("The PIN was accepted. {instruction}")
    } else {
        instruction.to_owned()
    }
}

fn security_key_uv_instruction(context: Option<&AuthenticatorPromptPurpose>) -> String {
    let instruction = match context {
        Some(AuthenticatorPromptPurpose::CreatePasskey) => text::SECURITY_KEY_CREATE_VERIFICATION,
        Some(AuthenticatorPromptPurpose::AuthorizeWithNewPasskey) => {
            text::SECURITY_KEY_AUTHORIZE_NEW_VERIFICATION
        }
        Some(AuthenticatorPromptPurpose::FindExistingPasskey) => {
            text::SECURITY_KEY_FIND_VERIFICATION
        }
        Some(AuthenticatorPromptPurpose::RecoverPasskeyPublicKey) => {
            text::SECURITY_KEY_VERIFY_VERIFICATION
        }
        Some(AuthenticatorPromptPurpose::AuthorizeWithExistingPasskey) => {
            text::SECURITY_KEY_AUTHORIZE_EXISTING_VERIFICATION
        }
        _ => text::SECURITY_KEY_DEFAULT_VERIFICATION,
    };
    format!("{instruction} Use its built-in verification, such as its fingerprint sensor.")
}

#[cfg(test)]
mod tests {
    use super::*;

    keyringd_testkit::scenario! {
        group "operation_prompt_text" {
            case "Presence copy keeps protection change distinct from replacement and preserves retry" {
                verify "render operation-specific security-key text and retain the retry guidance" {
                    use crate::key_operation::KeyOperationKind;
                    let (_, protection) = operation_prompt_text(
                        &KeyOperationKind::DeviceKeyProtectionChange {
                            profile_name: "Work".into(),
                        },
                        AuthenticatorPromptKind::UserPresence,
                        false,
                    );
                    assert!(protection.contains("Local authentication change for “Work”"));
                    assert!(!protection.contains("replacement"));
                    let (retry_title, retry_explanation) = operation_prompt_text(
                        &KeyOperationKind::AgentRequest,
                        AuthenticatorPromptKind::UserPresence,
                        true,
                    );
                    assert!(retry_title.contains("again"));
                    assert!(retry_explanation.contains("did not respond in time"));
                    let (_, uv) = operation_prompt_text(
                        &KeyOperationKind::AgentRequest,
                        AuthenticatorPromptKind::BuiltInUserVerification,
                        false,
                    );
                    assert!(uv.contains("fingerprint sensor"));
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "pin" {
            case "Pin copy retains minimum and attempts for the current step" {
                verify "include the setup purpose, PIN length, and attempts left in the prompt" {
                    let request = SecurityKeyPinPrompt {
                        reason: "Enter PIN".into(),
                        attempts_left: Some(2),
                        minimum_length: Some(6),
                        new_pin: false,
                    };
                    let (_, explanation) = pin(
                        &request,
                        Some(&AuthenticatorPromptPurpose::AuthorizeWithExistingPasskey),
                    );
                    assert!(explanation.contains("one last time"));
                    assert!(explanation.contains("at least 6 characters"));
                    assert!(explanation.contains("2 attempts remaining"));
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "security_key_touch_instruction" {
            case "Repeated security key touches explain the sequence" {
                verify "give different instructions for selecting the key and approving the request" {
                    assert!(
                        security_key_touch_instruction(
                            Some(&AuthenticatorPromptPurpose::AuthorizeWithNewPasskey),
                            false,
                        )
                        .contains("again")
                    );
                    assert!(
                        security_key_touch_instruction(
                            Some(&AuthenticatorPromptPurpose::AuthorizeWithExistingPasskey),
                            true,
                        )
                        .contains("one last time")
                    );
                }
            }
        }
    }
}
