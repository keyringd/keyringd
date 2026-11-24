use super::{PinPromptCommand, PinPromptEvent, SecurityKeyPinPrompt};
use crate::ComponentModel;
use keyringd_fido::ctap::Pin;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PinPromptState {
    pin: String,
    confirmation: String,
    minimum_length: u64,
    new_pin: bool,
    completed: bool,
}

impl Default for PinPromptState {
    fn default() -> Self {
        Self::new(4)
    }
}

impl PinPromptState {
    pub fn new(minimum_length: u64) -> Self {
        Self {
            pin: String::new(),
            confirmation: String::new(),
            minimum_length,
            new_pin: false,
            completed: false,
        }
    }

    pub fn for_prompt(prompt: &SecurityKeyPinPrompt) -> Self {
        Self {
            new_pin: prompt.new_pin,
            ..Self::new(u64::from(prompt.minimum_length.unwrap_or(4)))
        }
    }

    pub fn pin(&self) -> &str {
        &self.pin
    }

    pub fn confirmation(&self) -> &str {
        &self.confirmation
    }

    pub fn requires_confirmation(&self) -> bool {
        self.new_pin
    }

    pub fn can_submit(&self) -> bool {
        !self.completed && self.validation().0
    }

    pub fn validation_message(&self) -> Option<&'static str> {
        self.validation().1
    }

    fn validation(&self) -> (bool, Option<&'static str>) {
        let valid = Pin::from_text_with_minimum(&self.pin, self.minimum_length).is_ok();
        let matches = !self.new_pin || self.confirmation == self.pin;
        let message = if self.new_pin && !self.confirmation.is_empty() && !matches {
            Some("PINs do not match.")
        } else if !self.pin.is_empty() && !valid {
            Some("The PIN does not meet the authenticator's length requirements.")
        } else {
            None
        };
        (valid && matches, message)
    }
}

impl ComponentModel for PinPromptState {
    type Event = PinPromptEvent;
    type Command = PinPromptCommand;

    fn update(&mut self, event: Self::Event) -> Option<Self::Command> {
        if self.completed {
            return None;
        }
        match event {
            PinPromptEvent::PinChanged(pin) => self.pin = pin,
            PinPromptEvent::ConfirmationChanged(confirmation) => self.confirmation = confirmation,
            PinPromptEvent::SubmitRequested if self.can_submit() => {
                self.completed = true;
                self.confirmation.clear();
                return Some(PinPromptCommand::Submit(std::mem::take(&mut self.pin)));
            }
            PinPromptEvent::CancelRequested => {
                self.completed = true;
                self.pin.clear();
                self.confirmation.clear();
                return Some(PinPromptCommand::Cancel);
            }
            PinPromptEvent::Dismissed => {
                self.completed = true;
                self.pin.clear();
                self.confirmation.clear();
                return Some(PinPromptCommand::Dismiss);
            }
            PinPromptEvent::SubmitRequested => {}
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    keyringd_testkit::scenario! {
        group "PinPromptState::update" {
            case "Existing pin respects authenticator requirements and submits latest input" {
                verify "enable submit only for a valid PIN and send the most recent input" {
                    let mut state = PinPromptState::new(6);
                    for pin in ["", "12345", "123456\0"] {
                        state.update(PinPromptEvent::PinChanged(pin.into()));
                        assert!(!state.can_submit());
                        assert_eq!(state.update(PinPromptEvent::SubmitRequested), None);
                    }
                    state.update(PinPromptEvent::PinChanged("123456".into()));
                    assert!(state.can_submit());
                    assert_eq!(
                        state.update(PinPromptEvent::SubmitRequested),
                        Some(PinPromptCommand::Submit("123456".into()))
                    );
                    assert!(!state.can_submit());
                    assert_eq!(state.update(PinPromptEvent::SubmitRequested), None);
                    assert_eq!(state.update(PinPromptEvent::CancelRequested), None);
                    assert_eq!(state.update(PinPromptEvent::Dismissed), None);
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "PinPromptState::update" {
            case "New pin cannot submit until confirmation matches the current pin" {
                verify "enable new PIN submission after both fields match" {
                    let mut state = PinPromptState::for_prompt(&SecurityKeyPinPrompt {
                        reason: String::new(),
                        attempts_left: None,
                        minimum_length: Some(6),
                        new_pin: true,
                    });
                    assert!(state.requires_confirmation());
                    state.update(PinPromptEvent::PinChanged("123456".into()));
                    assert_eq!(state.update(PinPromptEvent::SubmitRequested), None);
                    state.update(PinPromptEvent::ConfirmationChanged("654321".into()));
                    assert!(state.validation_message().is_some());
                    assert!(!state.can_submit());
                    state.update(PinPromptEvent::ConfirmationChanged("123456".into()));
                    assert!(state.can_submit());
                    assert_eq!(state.validation_message(), None);
                    state.update(PinPromptEvent::PinChanged("654321".into()));
                    assert!(!state.can_submit());
                    state.update(PinPromptEvent::ConfirmationChanged("654321".into()));
                    assert_eq!(
                        state.update(PinPromptEvent::SubmitRequested),
                        Some(PinPromptCommand::Submit("654321".into()))
                    );
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "PinPromptState::update" {
            case "Cancellation and dismissal are terminal even with invalid input" {
                verify "consume cancel or dismiss once even when the PIN is invalid" {
                    for (event, command) in [
                        (PinPromptEvent::CancelRequested, PinPromptCommand::Cancel),
                        (PinPromptEvent::Dismissed, PinPromptCommand::Dismiss),
                    ] {
                        let mut state = PinPromptState::default();
                        state.update(PinPromptEvent::PinChanged("12".into()));
                        assert_eq!(state.update(event), Some(command));
                        state.update(PinPromptEvent::PinChanged("123456".into()));
                        assert!(state.pin().is_empty());
                        assert!(!state.can_submit());
                        assert_eq!(state.update(PinPromptEvent::Dismissed), None);
                    }
                }
            }
        }
    }
}
