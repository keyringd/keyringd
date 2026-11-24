use crate::ComponentModel;

use super::QrPromptCommand;
use super::QrPromptEvent;

#[derive(Clone, Default, PartialEq)]
pub struct QrPromptState {
    completed: bool,
}

impl ComponentModel for QrPromptState {
    type Event = QrPromptEvent;
    type Command = QrPromptCommand;

    fn update(&mut self, event: Self::Event) -> Option<Self::Command> {
        match event {
            QrPromptEvent::UseUsbRequested if !self.completed => Some(QrPromptCommand::UseUsb),
            QrPromptEvent::CancelRequested if !self.completed => {
                self.completed = true;
                Some(QrPromptCommand::Cancel)
            }
            QrPromptEvent::UsbSelected => {
                self.completed = true;
                None
            }
            QrPromptEvent::Dismissed => {
                self.completed = true;
                Some(QrPromptCommand::Close)
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    keyringd_testkit::scenario! {
        group "QrPromptState::update" {
            case "Rejected usb request can still be cancelled but only once" {
                verify "allow one cancellation after a USB switch request is rejected" {
                    let mut state = QrPromptState::default();
                    assert!(matches!(
                        state.update(QrPromptEvent::UseUsbRequested),
                        Some(QrPromptCommand::UseUsb)
                    ));
                    assert!(matches!(
                        state.update(QrPromptEvent::CancelRequested),
                        Some(QrPromptCommand::Cancel)
                    ));
                    assert!(state.update(QrPromptEvent::CancelRequested).is_none());
                    assert!(state.update(QrPromptEvent::UseUsbRequested).is_none());
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "QrPromptState::update" {
            case "Programmatic dismissal never becomes user cancellation" {
                verify "suppress cancel dispatch when the application dismisses the QR window" {
                    let mut state = QrPromptState::default();
                    assert!(matches!(
                        state.update(QrPromptEvent::Dismissed),
                        Some(QrPromptCommand::Close)
                    ));
                    assert!(state.update(QrPromptEvent::CancelRequested).is_none());
                    assert!(state.update(QrPromptEvent::UseUsbRequested).is_none());
                }
            }
        }
    }
    keyringd_testkit::scenario! {
        group "QrPromptState::update" {
            case "Completed QR Prompt ignores Cancel" {
                given "There is a QR Prompt selected and completed with the USB Security Key" {
                    let mut state = QrPromptState::default();
                    state.update(QrPromptEvent::UsbSelected);
                }
                when "Request Cancel" {
                    let command = state.update(QrPromptEvent::CancelRequested);
                }
                then "Do not issue additional commands" {
                    assert!(command.is_none());
                }
            }
            case "Close Dismiss QR Prompt even after selecting USB" {
                given "There is a QR Prompt with a USB Security Key selected" {
                    let mut state = QrPromptState::default();
                    state.update(QrPromptEvent::UsbSelected);
                }
                when "Prompt is dismissed" {
                    let command = state.update(QrPromptEvent::Dismissed);
                }
                then "Issue a command to close the window" {
                    assert!(matches!(command, Some(QrPromptCommand::Close)));
                }
            }
        }
    }
}
