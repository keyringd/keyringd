use super::{ProfileSetupInteractionScreen, ProfileSetupStep};
use crate::key_operation::AuthenticatorPromptPurpose;
use crate::text;
use keyringd_fido_client::OperationId;

/// A snapshot of the operation area, without access to the wizard's lifecycle.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProfileSetupInteractionPresentation {
    pub screen: ProfileSetupInteractionScreen,
    pub context: Option<AuthenticatorPromptPurpose>,
    pub stage: Option<ProfileSetupStep>,
    pub operation_id: Option<OperationId>,
    pub status: &'static str,
}

impl Default for ProfileSetupInteractionPresentation {
    fn default() -> Self {
        Self {
            screen: ProfileSetupInteractionScreen::Hidden,
            context: None,
            stage: None,
            operation_id: None,
            status: text::PROFILE_SETUP_PREPARING,
        }
    }
}

impl ProfileSetupInteractionPresentation {
    pub fn stage_title(&self) -> String {
        self.context
            .as_ref()
            .map(crate::qr_prompt_window::heading)
            .unwrap_or(text::PROFILE_SETUP_PREPARING_NEXT)
            .into()
    }

    pub fn stage_description(&self) -> String {
        self.context
            .as_ref()
            .map(crate::qr_prompt_window::description)
            .unwrap_or_else(|| self.status.into())
    }
    pub fn title(&self) -> String {
        match &self.screen {
            ProfileSetupInteractionScreen::Qr(qr) => {
                crate::qr_prompt_window::heading(&qr.context).into()
            }
            ProfileSetupInteractionScreen::Cancelling => text::PROFILE_SETUP_CANCELLING.into(),
            ProfileSetupInteractionScreen::Preparing => text::PROFILE_SETUP_PREPARING_NEXT.into(),
            _ => self.security_key_text().0.into(),
        }
    }

    pub fn description(&self) -> String {
        match &self.screen {
            ProfileSetupInteractionScreen::Qr(qr) => {
                crate::qr_prompt_window::description(&qr.context)
            }
            ProfileSetupInteractionScreen::Cancelling
            | ProfileSetupInteractionScreen::Preparing => self.status.into(),
            _ => self.security_key_text().1,
        }
    }

    pub fn pin_prompt(&self) -> Option<crate::pin_prompt_window::SecurityKeyPinPrompt> {
        let ProfileSetupInteractionScreen::Pin(request) = &self.screen else {
            return None;
        };
        let mut prompt = request.clone();
        prompt.reason = crate::security_key_window::pin_instruction(request, self.context.as_ref());
        Some(prompt)
    }

    fn security_key_text(&self) -> (&'static str, String) {
        use crate::security_key_window;
        match &self.screen {
            ProfileSetupInteractionScreen::Pin(request) => {
                security_key_window::pin(request, self.context.as_ref())
            }
            ProfileSetupInteractionScreen::Waiting {
                transport,
                prompt_kind,
                pin_requested,
            } => security_key_window::waiting(
                transport.as_ref(),
                *prompt_kind,
                *pin_requested,
                self.context.as_ref(),
            ),
            _ => security_key_window::waiting(None, None, false, self.context.as_ref()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    keyringd_testkit::scenario! {
        group "ProfileSetupInteractionPresentation::pin_prompt" {
            case "Pin prompt uses the setup step and preserves input constraints" {
                verify "render the setup-specific PIN reason with the authenticator's input constraints" {
                    let request = crate::pin_prompt_window::SecurityKeyPinPrompt {
                        reason: "Backend instruction".into(),
                        attempts_left: Some(2),
                        minimum_length: Some(6),
                        new_pin: false,
                    };
                    let mut presentation = ProfileSetupInteractionPresentation {
                        screen: ProfileSetupInteractionScreen::Pin(request.clone()),
                        context: Some(AuthenticatorPromptPurpose::CreatePasskey),
                        ..Default::default()
                    };
                    let creation = presentation.pin_prompt().unwrap();
                    presentation.context = Some(AuthenticatorPromptPurpose::AuthorizeWithExistingPasskey);
                    let authorization = presentation.pin_prompt().unwrap();
                    assert!(creation.reason.contains("create the Passkey"));
                    assert!(authorization.reason.contains("authorize this computer"));
                    for prompt in [creation, authorization] {
                        assert_eq!(prompt.attempts_left, request.attempts_left);
                        assert_eq!(prompt.minimum_length, request.minimum_length);
                        assert_eq!(prompt.new_pin, request.new_pin);
                    }
                }
            }
        }
    }
}
