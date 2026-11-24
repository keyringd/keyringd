use crate::text;
use keyringd_core::application::ProfileSetupFailure;
use keyringd_core::application::ProfileSetupRequest;
use keyringd_core::application::{PasskeyChoice, ProfileSetupProgress};
use keyringd_core::model::Profile;
use keyringd_ssh_agent_protocol::SshSignatureAlgorithm;

use super::{
    EditingPage, HybridPrompt, PasskeyCredentialSelection, PasskeyNotFoundChoice,
    ProfileSetupEvent, ProfileSetupInteractionPresentation, ProfileSetupInteractionScreen,
    ProfileSetupNotice, ProfileSetupPrompt, ProfileSetupScreen, ProfileSetupWindowCommand,
    ProfileSetupWindowEvent, TransportPrompts,
};
use crate::ComponentModel;
use crate::key_operation::AuthenticatorPromptKind;
use crate::key_operation::AuthenticatorPromptPurpose;
use crate::key_operation::KeyOperationCommand;
use crate::pin_prompt_window::PinPromptCommand;
use keyringd_core::application::ApplicationError;
use keyringd_fido::AuthenticatorTransport;
use keyringd_fido_client::InteractionId;

fn transport_prompts(prompt: &mut ProfileSetupPrompt) -> &mut TransportPrompts {
    if !matches!(prompt, ProfileSetupPrompt::Transports(_)) {
        let pin_requested = matches!(prompt, ProfileSetupPrompt::Pin { .. });
        *prompt = ProfileSetupPrompt::Transports(TransportPrompts {
            pin_requested,
            ..Default::default()
        });
    }
    match prompt {
        ProfileSetupPrompt::Transports(prompts) => prompts,
        _ => unreachable!(),
    }
}

/// Owns the setup window's form, running interaction, and completion state.
#[derive(Clone)]
pub enum ProfileSetupWindowState {
    Editing {
        display_name: String,
        passkey: PasskeyChoice,
        signature_algorithm: SshSignatureAlgorithm,
        advanced_settings: bool,
        notice: Option<ProfileSetupNotice>,
        page: EditingPage,
    },
    Running {
        display_name: String,
        passkey: PasskeyChoice,
        signature_algorithm: SshSignatureAlgorithm,
        advanced_settings: bool,
        progress: ProfileSetupProgress,
        prompt: ProfileSetupPrompt,
    },
    Completed {
        profile: Box<Profile>,
        window_error: Option<String>,
    },
}

impl PartialEq for ProfileSetupWindowState {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (
                Self::Editing {
                    display_name: a_display_name,
                    passkey: a_passkey,
                    signature_algorithm: a_signature_algorithm,
                    advanced_settings: a_advanced_settings,
                    notice: a_notice,
                    page: a_page,
                },
                Self::Editing {
                    display_name: b_display_name,
                    passkey: b_passkey,
                    signature_algorithm: b_signature_algorithm,
                    advanced_settings: b_advanced_settings,
                    notice: b_notice,
                    page: b_page,
                },
            ) => {
                a_display_name == b_display_name
                    && a_passkey == b_passkey
                    && a_signature_algorithm == b_signature_algorithm
                    && a_advanced_settings == b_advanced_settings
                    && a_notice == b_notice
                    && a_page == b_page
            }
            (
                Self::Running {
                    display_name: a_display_name,
                    passkey: a_passkey,
                    signature_algorithm: a_signature_algorithm,
                    advanced_settings: a_advanced_settings,
                    progress: a_progress,
                    prompt: a_prompt,
                },
                Self::Running {
                    display_name: b_display_name,
                    passkey: b_passkey,
                    signature_algorithm: b_signature_algorithm,
                    advanced_settings: b_advanced_settings,
                    progress: b_progress,
                    prompt: b_prompt,
                },
            ) => {
                a_display_name == b_display_name
                    && a_passkey == b_passkey
                    && a_signature_algorithm == b_signature_algorithm
                    && a_advanced_settings == b_advanced_settings
                    && a_progress == b_progress
                    && a_prompt == b_prompt
            }
            // A completed setup is an immutable snapshot of this newly created profile.
            (
                Self::Completed {
                    profile: a,
                    window_error: a_error,
                },
                Self::Completed {
                    profile: b,
                    window_error: b_error,
                },
            ) => a.id() == b.id() && a_error == b_error,
            _ => false,
        }
    }
}

impl ProfileSetupWindowState {
    pub fn name(&self) -> &str {
        match self {
            Self::Editing { display_name, .. } | Self::Running { display_name, .. } => display_name,
            Self::Completed { profile, .. } => profile.display_name(),
        }
    }

    pub fn is_editing(&self) -> bool {
        matches!(self, Self::Editing { .. })
    }

    pub fn editing() -> Self {
        Self::Editing {
            display_name: String::new(),
            passkey: PasskeyChoice::CreateNew,
            signature_algorithm: SshSignatureAlgorithm::EcdsaSha2NistP256,
            advanced_settings: false,
            notice: None,
            page: EditingPage::Form,
        }
    }

    pub fn signature_algorithm(&self) -> SshSignatureAlgorithm {
        match self {
            Self::Editing {
                signature_algorithm,
                ..
            }
            | Self::Running {
                signature_algorithm,
                ..
            } => signature_algorithm.clone(),
            Self::Completed { profile, .. } => profile.device_key().signature_algorithm(),
        }
    }
    pub fn advanced_settings(&self) -> bool {
        matches!(
            self,
            Self::Editing {
                advanced_settings: true,
                ..
            }
        )
    }
    pub fn progress(&self) -> Option<ProfileSetupProgress> {
        match self {
            Self::Running { progress, .. } => Some(*progress),
            Self::Editing { .. } | Self::Completed { .. } => None,
        }
    }

    pub fn operation_purpose(&self) -> Option<AuthenticatorPromptPurpose> {
        match self {
            Self::Running {
                progress, passkey, ..
            } => AuthenticatorPromptPurpose::for_profile_setup(*progress, *passkey),
            Self::Editing { .. } | Self::Completed { .. } => None,
        }
    }

    pub fn notice(&self) -> Option<&ProfileSetupNotice> {
        match self {
            Self::Editing { notice, .. } => notice.as_ref(),
            Self::Running { .. } | Self::Completed { .. } => None,
        }
    }

    pub fn editing_page(&self) -> Option<EditingPage> {
        match self {
            Self::Editing { page, .. } => Some(*page),
            _ => None,
        }
    }

    pub fn screen(&self) -> ProfileSetupScreen {
        if matches!(
            self,
            Self::Running {
                prompt: ProfileSetupPrompt::Cancelling { .. },
                ..
            }
        ) {
            ProfileSetupScreen::Cancelling
        } else if self.completed_profile().is_some() {
            ProfileSetupScreen::Completed
        } else if self.credential_selection().is_some() {
            ProfileSetupScreen::Credentials
        } else if matches!(self.editing_page(), Some(EditingPage::PasskeyNotFound(_))) {
            ProfileSetupScreen::PasskeyNotFound
        } else if self.is_running() {
            ProfileSetupScreen::Interaction
        } else {
            ProfileSetupScreen::Form
        }
    }

    pub fn can_start(&self) -> bool {
        matches!(self, Self::Editing { display_name, page: EditingPage::Form, .. } if !display_name.trim().is_empty())
    }

    pub fn submit_label(&self) -> &'static str {
        if self.error().is_some() {
            crate::text::PROFILE_SETUP_RETRY
        } else {
            crate::text::PROFILE_SETUP_CREATE
        }
    }

    pub fn can_go_back(&self) -> bool {
        self.screen() != ProfileSetupScreen::Cancelling
            && (self.is_running()
                || matches!(self.editing_page(), Some(EditingPage::PasskeyNotFound(_))))
    }

    pub fn is_running(&self) -> bool {
        matches!(self, Self::Running { .. })
    }

    pub fn passkey(&self) -> PasskeyChoice {
        match self {
            Self::Editing { passkey, .. } | Self::Running { passkey, .. } => *passkey,
            Self::Completed { .. } => PasskeyChoice::CreateNew,
        }
    }

    #[cfg(test)]
    pub fn prompt(&self) -> &ProfileSetupPrompt {
        match self {
            Self::Running { prompt, .. } => prompt,
            Self::Editing { .. } | Self::Completed { .. } => &ProfileSetupPrompt::Hidden,
        }
    }

    fn interaction_screen(&self) -> ProfileSetupInteractionScreen {
        use ProfileSetupInteractionScreen as Screen;
        let Self::Running { prompt, .. } = self else {
            return Screen::Hidden;
        };
        match prompt {
            ProfileSetupPrompt::Hidden => Screen::Preparing,
            ProfileSetupPrompt::CredentialSelection(_) => Screen::Hidden,
            ProfileSetupPrompt::Cancelling { .. } => Screen::Cancelling,
            ProfileSetupPrompt::Pin { request, .. } => Screen::Pin(request.clone()),
            ProfileSetupPrompt::Preparing => Screen::Waiting {
                transport: None,
                prompt_kind: None,
                pin_requested: false,
            },
            ProfileSetupPrompt::Transports(prompts) => {
                // Once USB has been chosen, a still-active Hybrid QR must not
                // obscure its authenticator prompt.
                if prompts.usb_requested
                    || prompts
                        .selected_transport
                        .as_ref()
                        .is_some_and(|value| value.as_str() == "usb")
                {
                    return Screen::Waiting {
                        transport: Some(AuthenticatorTransport::usb()),
                        prompt_kind: prompts.prompt_kind,
                        pin_requested: prompts.pin_requested,
                    };
                }
                if let Some(hybrid) = &prompts.hybrid {
                    return Screen::Qr(hybrid.clone());
                }
                Screen::Waiting {
                    transport: prompts.prompted_transports.first().cloned(),
                    prompt_kind: prompts.prompt_kind,
                    pin_requested: prompts.pin_requested,
                }
            }
        }
    }

    pub fn credential_selection(&self) -> Option<&PasskeyCredentialSelection> {
        match self {
            Self::Running {
                prompt: ProfileSetupPrompt::CredentialSelection(selection),
                ..
            } => Some(selection),
            _ => None,
        }
    }

    pub fn completed_profile(&self) -> Option<&Profile> {
        match self {
            Self::Completed { profile, .. } => Some(profile),
            Self::Editing { .. } | Self::Running { .. } => None,
        }
    }

    pub fn progress_message(&self) -> Option<&'static str> {
        self.progress()
            .map(crate::profile_setup_window::progress_message)
    }

    pub fn interaction_presentation(&self) -> ProfileSetupInteractionPresentation {
        use crate::profile_setup_window::ProfileSetupStep;
        let screen = self.interaction_screen();
        let context = match &screen {
            ProfileSetupInteractionScreen::Qr(qr) => Some(qr.context.clone()),
            _ => self.operation_purpose(),
        };
        let operation_id = match &screen {
            ProfileSetupInteractionScreen::Qr(qr) => Some(qr.operation_id),
            ProfileSetupInteractionScreen::Pin(_) => {
                self.pin_request_id().map(InteractionId::operation_id)
            }
            _ => None,
        };
        let stage = context
            .as_ref()
            .and_then(ProfileSetupStep::for_purpose)
            .or_else(|| {
                self.progress()
                    .map(|progress| ProfileSetupStep::for_progress(progress, self.passkey()))
            });
        let status = if matches!(screen, ProfileSetupInteractionScreen::Cancelling) {
            text::PROFILE_SETUP_CANCELLING_DESCRIPTION
        } else {
            self.progress_message()
                .unwrap_or(text::PROFILE_SETUP_PREPARING)
        };
        ProfileSetupInteractionPresentation {
            screen,
            context,
            stage,
            operation_id,
            status,
        }
    }

    pub fn notice_message(&self) -> Option<&str> {
        match self {
            Self::Editing { notice, .. } => notice.as_ref().map(ProfileSetupNotice::message),
            Self::Completed { window_error, .. } => window_error.as_deref(),
            _ => None,
        }
    }
}

impl Default for ProfileSetupWindowState {
    fn default() -> Self {
        Self::editing()
    }
}

impl ComponentModel for ProfileSetupWindowState {
    type Event = ProfileSetupWindowEvent;
    type Command = ProfileSetupWindowCommand;

    fn update(&mut self, event: Self::Event) -> Option<Self::Command> {
        use ProfileSetupWindowEvent as Event;
        match event {
            Event::Input(event) => self.input(event),
            Event::KeyOperation(command) => self.key_operation(command),
            Event::Finished(result) => self.finish(result),
            Event::TrustEntriesOpened(result) => {
                if let Self::Completed { window_error, .. } = self {
                    *window_error = result.err();
                }
                None
            }
            Event::Progressed(progress) => {
                if self.accepts_interaction()
                    && let Self::Running {
                        progress: current,
                        prompt,
                        ..
                    } = self
                {
                    *current = progress;
                    *prompt = ProfileSetupPrompt::Preparing;
                }
                None
            }
            Event::CredentialSelectionRequested(candidates) => {
                if self.accepts_interaction()
                    && let Self::Running { prompt, .. } = self
                {
                    *prompt = ProfileSetupPrompt::CredentialSelection(
                        PasskeyCredentialSelection::new(candidates),
                    );
                }
                None
            }
            Event::SecurityKeyActivationRejected => {
                if let Self::Running {
                    prompt: ProfileSetupPrompt::Transports(prompts),
                    ..
                } = self
                {
                    prompts.usb_requested = false;
                }
                None
            }
            Event::PinRequested {
                request_id,
                prompt: request,
            } => {
                if self.accepts_interaction()
                    && let Self::Running { prompt, .. } = self
                {
                    *prompt = ProfileSetupPrompt::Pin {
                        request_id,
                        request,
                    };
                }
                None
            }
        }
    }
}

impl ProfileSetupWindowState {
    fn accepts_interaction(&self) -> bool {
        matches!(self, Self::Running { prompt, .. }
            if !matches!(prompt, ProfileSetupPrompt::Cancelling { .. }))
    }

    pub fn is_closing(&self) -> bool {
        matches!(
            self,
            Self::Running {
                prompt: ProfileSetupPrompt::Cancelling { close_window: true },
                ..
            }
        )
    }

    pub fn pin_request_id(&self) -> Option<InteractionId> {
        match self {
            Self::Running {
                prompt: ProfileSetupPrompt::Pin { request_id, .. },
                ..
            } => Some(*request_id),
            _ => None,
        }
    }

    pub fn cancelled(&self) -> bool {
        matches!(self.notice(), Some(ProfileSetupNotice::Cancelled))
    }

    pub fn error(&self) -> Option<&str> {
        self.notice()
            .filter(|notice| notice.is_error())
            .map(ProfileSetupNotice::message)
    }

    pub fn completed_id(&self) -> Option<&keyringd_core::model::ProfileId> {
        self.completed_profile().map(Profile::id)
    }

    fn input(&mut self, event: ProfileSetupEvent) -> Option<ProfileSetupWindowCommand> {
        use ProfileSetupEvent as Event;
        use ProfileSetupWindowCommand as Command;
        if matches!(
            self,
            Self::Running {
                prompt: ProfileSetupPrompt::Cancelling { .. },
                ..
            }
        ) {
            return None;
        }
        match event {
            Event::NameChanged(name) => {
                if let Self::Editing {
                    display_name,
                    notice,
                    ..
                } = self
                    && *display_name != name
                {
                    *display_name = name;
                    if !matches!(
                        notice,
                        Some(ProfileSetupNotice::NewPasskeyAfterFailedRecovery)
                    ) {
                        *notice = None;
                    }
                }
            }
            Event::PasskeyChanged(choice) => {
                if let Self::Editing {
                    passkey, notice, ..
                } = self
                    && *passkey != choice
                {
                    *passkey = choice;
                    if choice == PasskeyChoice::Existing
                        || !matches!(
                            notice,
                            Some(ProfileSetupNotice::NewPasskeyAfterFailedRecovery)
                        )
                    {
                        *notice = None;
                    }
                }
            }
            Event::SignatureAlgorithmChanged(format) => {
                if let Self::Editing {
                    signature_algorithm,
                    ..
                } = self
                {
                    *signature_algorithm = format;
                }
            }
            Event::AdvancedSettingsToggled => {
                if let Self::Editing {
                    advanced_settings, ..
                } = self
                {
                    *advanced_settings = !*advanced_settings;
                }
            }
            Event::SetupRequested
            | Event::CredentialSelectionConfirmed
            | Event::PasskeyNotFoundChoiceConfirmed => return self.primary(),
            Event::PasskeyNotFoundChoiceChanged(choice) => {
                if let Self::Editing { page, .. } = self
                    && matches!(page, EditingPage::PasskeyNotFound(_))
                {
                    *page = EditingPage::PasskeyNotFound(choice);
                }
            }
            Event::BackToSetup | Event::BackRequested => match self {
                Self::Running { prompt, .. } => {
                    *prompt = ProfileSetupPrompt::Cancelling {
                        close_window: false,
                    };
                    return Some(Command::CancelOperation);
                }
                Self::Editing { page, notice, .. } => {
                    *page = EditingPage::Form;
                    *notice = None;
                }
                Self::Completed { .. } => {}
            },
            Event::Dismissed => {
                if let Self::Running { prompt, .. } = self {
                    *prompt = ProfileSetupPrompt::Cancelling { close_window: true };
                    return Some(Command::CancelOperation);
                }
                return Some(Command::CloseWindow);
            }
            Event::ShowTrustEntries => {
                return self.completed_id().cloned().map(Command::OpenTrustEntries);
            }
            Event::CredentialChoiceChanged(index) => {
                if let Self::Running {
                    prompt: ProfileSetupPrompt::CredentialSelection(selection),
                    ..
                } = self
                {
                    selection.select(index);
                }
            }
            Event::PinCompleted {
                request_id,
                command,
            } => {
                if self.pin_request_id() != Some(request_id) {
                    return None;
                }
                if let Self::Running { prompt, .. } = self {
                    *prompt = if matches!(command, PinPromptCommand::Submit(_)) {
                        ProfileSetupPrompt::Transports(TransportPrompts {
                            prompted_transports: vec![AuthenticatorTransport::usb()],
                            prompt_kind: Some(AuthenticatorPromptKind::UserPresence),
                            pin_requested: true,
                            ..Default::default()
                        })
                    } else {
                        ProfileSetupPrompt::Preparing
                    };
                }
                return Some(Command::RespondPin {
                    request_id,
                    command,
                });
            }
            Event::UseSecurityKeyRequested(operation_id) => {
                let ProfileSetupInteractionScreen::Qr(qr) = self.interaction_screen() else {
                    return None;
                };
                if qr.operation_id != operation_id
                    || !qr.security_key_available
                    || !qr.context.allows_security_key_choice()
                {
                    return None;
                }
                if let Self::Running {
                    prompt: ProfileSetupPrompt::Transports(prompts),
                    ..
                } = self
                {
                    prompts.usb_requested = true;
                    return Some(Command::UseUsb(operation_id));
                }
            }
        }
        None
    }

    fn primary(&mut self) -> Option<ProfileSetupWindowCommand> {
        use ProfileSetupWindowCommand as Command;
        match self {
            Self::Running { prompt, .. } => {
                if let ProfileSetupPrompt::CredentialSelection(selection) = prompt {
                    let selected = selection
                        .selected_id()
                        .ok_or(ApplicationError::OperationCanceled);
                    *prompt = ProfileSetupPrompt::Preparing;
                    return Some(Command::SelectCredential(selected));
                }
            }
            Self::Completed { .. } => return Some(Command::CloseWindow),
            Self::Editing {
                display_name,
                passkey,
                signature_algorithm,
                advanced_settings,
                notice,
                page,
            } => {
                if let EditingPage::PasskeyNotFound(choice) = page {
                    match choice {
                        PasskeyNotFoundChoice::TryAnotherDevice => *page = EditingPage::Form,
                        PasskeyNotFoundChoice::CreateNewPasskey => {
                            *passkey = PasskeyChoice::CreateNew;
                            *page = EditingPage::Form;
                            *notice = Some(ProfileSetupNotice::NewPasskeyAfterFailedRecovery);
                            return None;
                        }
                    }
                }
                if display_name.trim().is_empty() {
                    *notice = Some(ProfileSetupNotice::MissingName);
                    return None;
                }
                let mut request =
                    ProfileSetupRequest::new(display_name.trim().to_owned(), *passkey);
                request.device_key_signature_algorithm = signature_algorithm.clone();
                *self = Self::Running {
                    display_name: request.display_name.clone(),
                    passkey: request.passkey,
                    signature_algorithm: request.device_key_signature_algorithm.clone(),
                    advanced_settings: *advanced_settings,
                    progress: ProfileSetupProgress::Starting,
                    prompt: ProfileSetupPrompt::Hidden,
                };
                return Some(Command::Start(request));
            }
        }
        None
    }

    fn finish(
        &mut self,
        result: std::sync::Arc<Result<Profile, ProfileSetupFailure>>,
    ) -> Option<ProfileSetupWindowCommand> {
        let close_window = self.is_closing();
        let Self::Running {
            display_name,
            passkey,
            signature_algorithm,
            advanced_settings,
            ..
        } = self
        else {
            return None;
        };
        *self = match result.as_ref() {
            Ok(profile) => Self::Completed {
                profile: Box::new(profile.clone()),
                window_error: None,
            },
            Err(failure) => {
                let page = if matches!(failure, ProfileSetupFailure::PasskeyNotFound) {
                    EditingPage::PasskeyNotFound(PasskeyNotFoundChoice::default())
                } else {
                    EditingPage::Form
                };
                let notice = match failure {
                    ProfileSetupFailure::Cancelled => ProfileSetupNotice::Cancelled,
                    ProfileSetupFailure::PasskeyNotFound => ProfileSetupNotice::PasskeyNotFound,
                    ProfileSetupFailure::Failed(operation_error) => {
                        ProfileSetupNotice::OperationFailed(
                            operation_error.report_user_error(&operation_error.user_message()),
                        )
                    }
                };
                Self::Editing {
                    display_name: display_name.clone(),
                    passkey: *passkey,
                    signature_algorithm: signature_algorithm.clone(),
                    advanced_settings: *advanced_settings,
                    notice: Some(notice),
                    page,
                }
            }
        };
        Some(ProfileSetupWindowCommand::OperationSettled { close_window })
    }

    fn key_operation(&mut self, command: KeyOperationCommand) -> Option<ProfileSetupWindowCommand> {
        use KeyOperationCommand as Command;
        if let Command::AcknowledgeStaleHybridDismissal(acknowledged) = command {
            return Some(ProfileSetupWindowCommand::Acknowledge(acknowledged));
        }
        if matches!(command, Command::CloseOperationStatus) {
            if self.accepts_interaction()
                && let Self::Running { prompt, .. } = self
                && !matches!(prompt, ProfileSetupPrompt::CredentialSelection(_))
            {
                *prompt = ProfileSetupPrompt::Preparing;
            }
            return Some(ProfileSetupWindowCommand::KeyOperation(command));
        }
        if !self.accepts_interaction() {
            return match command {
                Command::DismissHybridQr { acknowledged, .. } => {
                    Some(ProfileSetupWindowCommand::Acknowledge(acknowledged))
                }
                _ => None,
            };
        }
        let Self::Running {
            prompt,
            progress,
            passkey,
            ..
        } = self
        else {
            return None;
        };
        // Credential selection belongs to the setup lifecycle, not an authenticator view.
        if matches!(prompt, ProfileSetupPrompt::CredentialSelection(_)) {
            return match command {
                Command::DismissHybridQr { acknowledged, .. } => {
                    Some(ProfileSetupWindowCommand::Acknowledge(acknowledged))
                }
                _ => None,
            };
        }
        match command {
            Command::UpdateOperation { transport } => {
                let prompts = transport_prompts(prompt);
                prompts.selected_transport = Some(transport);
                prompts.pin_requested = false;
            }
            Command::PresentHybridQr {
                prompt_id,
                value,
                context: _,
                security_key_available,
                presented,
                unavailable,
                ..
            } => {
                // Recovery can skip public-key confirmation when largeBlob supplies it.
                // The prompt purpose follows progress rather than the QR request count.
                let context = super::ProfileSetupStep::for_progress(*progress, *passkey).purpose();
                transport_prompts(prompt).hybrid = Some(HybridPrompt {
                    operation_id: prompt_id.operation_id(),
                    value,
                    context,
                    unavailable,
                    security_key_available,
                });
                return Some(ProfileSetupWindowCommand::Acknowledge(presented));
            }
            Command::UpdateHybridAvailability { reason } => {
                if let ProfileSetupPrompt::Transports(TransportPrompts {
                    hybrid: Some(hybrid),
                    ..
                }) = prompt
                {
                    hybrid.unavailable = Some(reason);
                }
            }
            Command::ShowAuthenticatorPrompt {
                transport,
                prompt_kind,
                ..
            } => {
                let prompts = transport_prompts(prompt);
                if transport.as_str() == "usb" {
                    prompts.prompt_kind = Some(prompt_kind);
                }
                if !prompts.prompted_transports.contains(&transport) {
                    prompts.prompted_transports.push(transport);
                }
            }
            Command::DismissHybridQr { acknowledged, .. } => {
                if !matches!(prompt, ProfileSetupPrompt::Pin { .. }) {
                    *prompt = ProfileSetupPrompt::Preparing;
                }
                return Some(ProfileSetupWindowCommand::Acknowledge(acknowledged));
            }
            Command::CancelOperation => return self.input(ProfileSetupEvent::BackRequested),
            Command::ActivateUsb(operation_id) => {
                return self.input(ProfileSetupEvent::UseSecurityKeyRequested(operation_id));
            }
            native => return Some(ProfileSetupWindowCommand::KeyOperation(native)),
        }
        None
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        key_operation::KeyOperationKind, pin_prompt_window::SecurityKeyPinPrompt,
        profile_setup_window::ProfileSetupInteractionScreen,
    };
    use keyringd_fido_client::{HybridTransportUnavailableReason, OperationId};
    use std::sync::Arc;

    keyringd_testkit::scenario! {
        group "ProfileSetupWindowState::update" {
            case "Interaction snapshot follows prompt changes and hides during selection" {
                verify "expose the current prompt in the snapshot and hide it during candidate selection" {
                    use crate::profile_setup_window::ProfileSetupInteractionScreen as Screen;
                    let mut state = ProfileSetupWindowState::editing();
                    assert!(matches!(
                        state.interaction_presentation().screen,
                        Screen::Hidden
                    ));
                    state.update(ProfileSetupEvent::NameChanged("Work".into()).into());
                    state.update(ProfileSetupEvent::SetupRequested.into());
                    state.update(ProfileSetupWindowEvent::Progressed(
                        ProfileSetupProgress::CreatingPasskey,
                    ));
                    state.update(key(KeyOperationCommand::PresentHybridQr { prompt_id: request_id(1), presented: tokio::sync::oneshot::channel().0, replace_active: false,
                            value: "FIDO:/test".into(),
                            context: None,
                            unavailable: None,
                            security_key_available: true,
                        }));
                    let presentation = state.interaction_presentation();
                    assert_eq!(
                        presentation.stage,
                        Some(crate::profile_setup_window::ProfileSetupStep::Create)
                    );
                    assert!(
                        matches!(presentation.screen, Screen::Qr(qr) if qr.value == "FIDO:/test" && qr.security_key_available)
                    );
                    state.update(ProfileSetupWindowEvent::PinRequested { request_id: request_id(1), prompt: crate::pin_prompt_window::SecurityKeyPinPrompt {
                            reason: "Enter PIN".into(),
                            attempts_left: Some(2),
                            minimum_length: Some(6),
                            new_pin: false,
                        } });
                    assert!(
                        matches!(state.interaction_presentation().screen, Screen::Pin(pin) if pin.minimum_length == Some(6) && pin.attempts_left == Some(2))
                    );
                    state.update(ProfileSetupWindowEvent::CredentialSelectionRequested(Vec::new()));
                    assert!(matches!(
                        state.interaction_presentation().screen,
                        Screen::Hidden
                    ));
                    state.update(ProfileSetupEvent::BackRequested.into());
                    assert!(matches!(
                        state.interaction_presentation().screen,
                        Screen::Cancelling
                    ));
                    assert_eq!(
                        state.interaction_presentation().status,
                        text::PROFILE_SETUP_CANCELLING_DESCRIPTION
                    );
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "ProfileSetupWindowState::update" {
            case "Passkey step context follows profile setup progress for both paths" {
                verify "associate each registration or recovery state with its expected Passkey prompt" {
                    use crate::key_operation::AuthenticatorPromptPurpose;

                    for (choice, steps) in [
                        (
                            PasskeyChoice::CreateNew,
                            vec![
                                (
                                    ProfileSetupProgress::CreatingPasskey,
                                    AuthenticatorPromptPurpose::CreatePasskey,
                                ),
                                (
                                    ProfileSetupProgress::ProvisioningDeviceKey,
                                    AuthenticatorPromptPurpose::AuthorizeWithNewPasskey,
                                ),
                            ],
                        ),
                        (
                            PasskeyChoice::Existing,
                            vec![
                                (
                                    ProfileSetupProgress::DiscoveringPasskey,
                                    AuthenticatorPromptPurpose::FindExistingPasskey,
                                ),
                                (
                                    ProfileSetupProgress::RecoveringPasskeyPublicKey,
                                    AuthenticatorPromptPurpose::RecoverPasskeyPublicKey,
                                ),
                                (
                                    ProfileSetupProgress::ProvisioningDeviceKey,
                                    AuthenticatorPromptPurpose::AuthorizeWithExistingPasskey,
                                ),
                            ],
                        ),
                    ] {
                        let mut state = ProfileSetupWindowState::editing();
                        state.update(ProfileSetupEvent::NameChanged("Work".to_owned()).into());
                        state.update(ProfileSetupEvent::PasskeyChanged(choice).into());
                        state.update(ProfileSetupEvent::SetupRequested.into());
                        assert_eq!(state.operation_purpose(), None);
                        for (progress, expected) in steps {
                            state.update(ProfileSetupWindowEvent::Progressed(progress));
                            assert_eq!(state.operation_purpose(), Some(expected));
                        }
                        state.update(ProfileSetupWindowEvent::Progressed(
                            ProfileSetupProgress::CommittingProfile,
                        ));
                        assert_eq!(state.operation_purpose(), None);
                    }
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "ProfileSetupWindowState::update" {
            case "Selected key presence replaces the pin prompt" {
                verify "show the chosen key's presence prompt and clear the preceding PIN prompt" {
                    let mut state = ProfileSetupWindowState::editing();
                    state.update(ProfileSetupEvent::NameChanged("Work".to_owned()).into());
                    state.update(ProfileSetupEvent::SetupRequested.into());
                    state.update(ProfileSetupWindowEvent::PinRequested { request_id: request_id(1), prompt: SecurityKeyPinPrompt {
                            reason: "Enter PIN".to_owned(),
                            attempts_left: Some(8),
                            minimum_length: None,
                            new_pin: false,
                        } });

                    state.update(key(KeyOperationCommand::ShowAuthenticatorPrompt { operation: KeyOperationKind::ProfileSetup { passkey: PasskeyChoice::CreateNew },
                            transport: AuthenticatorTransport::usb(),
                            prompt_kind: AuthenticatorPromptKind::UserPresence,
                        }));

                    assert!(matches!(
                        state.prompt(),
                        ProfileSetupPrompt::Transports(TransportPrompts { prompted_transports, pin_requested: true, .. })
                            if prompted_transports == &[AuthenticatorTransport::usb()]
                    ));
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "ProfileSetupWindowState::update" {
            case "Submitting pin shows security key presence" {
                verify "replace PIN entry with the touch-instruction screen after submit" {
                    let mut state = ProfileSetupWindowState::editing();
                    state.update(ProfileSetupEvent::NameChanged("Work".to_owned()).into());
                    state.update(ProfileSetupEvent::SetupRequested.into());
                    state.update(ProfileSetupWindowEvent::PinRequested { request_id: request_id(1), prompt: SecurityKeyPinPrompt {
                            reason: "Enter PIN".to_owned(),
                            attempts_left: Some(8),
                            minimum_length: None,
                            new_pin: false,
                        } });

                    state.update(ProfileSetupEvent::PinCompleted { request_id: request_id(1), command: PinPromptCommand::Submit("1234".into()) }.into());

                    assert!(matches!(
                        state.prompt(),
                        ProfileSetupPrompt::Transports(TransportPrompts {
                            prompted_transports,
                            pin_requested: true,
                            ..
                        }) if prompted_transports == &[AuthenticatorTransport::usb()]
                    ));
                    assert!(matches!(
                        state.interaction_presentation().screen,
                        ProfileSetupInteractionScreen::Waiting { transport: Some(transport), pin_requested: true, .. }
                            if transport.as_str() == "usb"
                    ));
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "ProfileSetupWindowState::update" {
            case "Choosing security key replaces qr page even when qr remains available" {
                verify "show security-key instructions immediately after the user switches from QR" {
                    let mut state = ProfileSetupWindowState::editing();
                    state.update(ProfileSetupEvent::NameChanged("Work".to_owned()).into());
                    state.update(ProfileSetupEvent::SetupRequested.into());
                    state.update(key(KeyOperationCommand::PresentHybridQr { prompt_id: request_id(1), presented: tokio::sync::oneshot::channel().0, replace_active: false,
                            value: "FIDO:/test".to_owned(),
                            context: None,
                            unavailable: None,
                            security_key_available: true,
                        }));
                    assert!(matches!(state.interaction_presentation().screen, ProfileSetupInteractionScreen::Qr(_)));

                    assert!(matches!(
                        state.update(ProfileSetupEvent::UseSecurityKeyRequested(
                            keyringd_fido_client::OperationId::new(1)
                        ).into()),
                        Some(ProfileSetupWindowCommand::UseUsb(operation_id)) if operation_id == keyringd_fido_client::OperationId::new(1)
                    ));
                    assert!(matches!(
                        state.interaction_presentation().screen,
                        ProfileSetupInteractionScreen::Waiting { transport: Some(transport), .. }
                            if transport.as_str() == "usb"
                    ));

                    state.update(ProfileSetupWindowEvent::SecurityKeyActivationRejected);
                    assert!(matches!(state.interaction_presentation().screen, ProfileSetupInteractionScreen::Qr(_)));
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "ProfileSetupWindowState::update" {
            case "Later hybrid steps cannot offer a security key switch" {
                verify "suppress USB switching after a Hybrid authenticator has been selected" {
                    let mut state = ProfileSetupWindowState::editing();
                    state.update(ProfileSetupEvent::NameChanged("Work".to_owned()).into());
                    state.update(ProfileSetupEvent::SetupRequested.into());
                    state.update(ProfileSetupWindowEvent::Progressed(
                        ProfileSetupProgress::ProvisioningDeviceKey,
                    ));
                    state.update(key(KeyOperationCommand::PresentHybridQr { prompt_id: request_id(1), presented: tokio::sync::oneshot::channel().0, replace_active: false,
                            value: "FIDO:/later".to_owned(),
                            context: None,
                            unavailable: None,
                            security_key_available: true,
                        }));
                    assert!(matches!(state.interaction_presentation().screen, ProfileSetupInteractionScreen::Qr(_)));
                    assert!(
                        state
                            .update(ProfileSetupEvent::UseSecurityKeyRequested(
                                keyringd_fido_client::OperationId::new(1)
                            ).into())
                            .is_none()
                    );
                    assert!(matches!(state.interaction_presentation().screen, ProfileSetupInteractionScreen::Qr(_)));
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "ProfileSetupWindowState::update" {
            case "Passkey not found choices keep retry and new key distinct" {
                verify "map the two choices to separate recovery and creation actions" {
                    let mut state = ProfileSetupWindowState::editing();
                    state.update(ProfileSetupEvent::NameChanged("Work".to_owned()).into());
                    state.update(ProfileSetupEvent::PasskeyChanged(PasskeyChoice::Existing).into());
                    state.update(ProfileSetupEvent::SetupRequested.into());
                    state.update(ProfileSetupWindowEvent::Finished(std::sync::Arc::new(Err(
                        ProfileSetupFailure::PasskeyNotFound,
                    ))));
                    assert_eq!(
                        state.editing_page(),
                        Some(EditingPage::PasskeyNotFound(
                            PasskeyNotFoundChoice::TryAnotherDevice
                        ))
                    );

                    let retry = state.update(ProfileSetupEvent::PasskeyNotFoundChoiceConfirmed.into());
                    assert!(matches!(
                        retry,
                        Some(ProfileSetupWindowCommand::Start(ProfileSetupRequest {
                            passkey: PasskeyChoice::Existing,
                            ..
                        }))
                    ));

                    state.update(ProfileSetupWindowEvent::Finished(std::sync::Arc::new(Err(
                        ProfileSetupFailure::PasskeyNotFound,
                    ))));
                    state.update(ProfileSetupEvent::PasskeyNotFoundChoiceChanged(
                        PasskeyNotFoundChoice::CreateNewPasskey,
                    ).into());
                    let create = state.update(ProfileSetupEvent::PasskeyNotFoundChoiceConfirmed.into());
                    assert!(create.is_none());
                    assert_eq!(state.editing_page(), Some(EditingPage::Form));
                    assert!(matches!(
                        state,
                        ProfileSetupWindowState::Editing {
                            passkey: PasskeyChoice::CreateNew,
                            ..
                        }
                    ));
                    assert!(matches!(
                        state.notice(),
                        Some(ProfileSetupNotice::NewPasskeyAfterFailedRecovery)
                    ));
                    state.update(ProfileSetupEvent::NameChanged("Work laptop".to_owned()).into());
                    assert!(matches!(
                        state.notice(),
                        Some(ProfileSetupNotice::NewPasskeyAfterFailedRecovery)
                    ));
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "ProfileSetupWindowState::update" {
            case "Start is synchronous and duplicate requests do not start another operation" {
                verify "enter the running state before dispatch and ignore a second start event" {
                    let mut state = ProfileSetupWindowState::editing();
                    state.update(ProfileSetupEvent::NameChanged(" Work ".into()).into());
                    assert!(matches!(
                        state.update(ProfileSetupEvent::SetupRequested.into()),
                        Some(ProfileSetupWindowCommand::Start(_))
                    ));
                    assert!(state.is_running());
                    assert!(state.update(ProfileSetupEvent::SetupRequested.into()).is_none());
                    state.update(ProfileSetupEvent::NameChanged("changed".into()).into());
                    assert_eq!(state.name(), "Work");
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "ProfileSetupWindowState::update" {
            case "Cancellation ignores late prompts until cleanup finishes" {
                verify "ignore new prompts after cancellation and wait for cleanup before leaving the running state" {
                    let mut state = ProfileSetupWindowState::editing();
                    state.update(ProfileSetupEvent::NameChanged("Work".into()).into());
                    state.update(ProfileSetupEvent::SetupRequested.into());
                    state.update(ProfileSetupEvent::BackRequested.into());
                    state.update(ProfileSetupWindowEvent::CredentialSelectionRequested(vec![]));
                    state.update(ProfileSetupWindowEvent::Progressed(
                        ProfileSetupProgress::CreatingPasskey,
                    ));
                    assert_eq!(state.screen(), ProfileSetupScreen::Cancelling);
                    assert!(state.credential_selection().is_none());
                    assert_eq!(state.progress(), Some(ProfileSetupProgress::Starting));
                    assert!(state.update(ProfileSetupEvent::SetupRequested.into()).is_none());
                    state.update(ProfileSetupWindowEvent::Finished(std::sync::Arc::new(Err(
                        ProfileSetupFailure::Cancelled,
                    ))));
                    assert_eq!(state.screen(), ProfileSetupScreen::Form);
                    assert_eq!(state.name(), "Work");
                    assert!(matches!(
                        state.notice(),
                        Some(ProfileSetupNotice::Cancelled)
                    ));
                    // A queued progress callback must not restore the old operation.
                    state.update(ProfileSetupWindowEvent::Progressed(
                        ProfileSetupProgress::CreatingPasskey,
                    ));
                    assert!(state.progress().is_none());
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "ProfileSetupWindowState::update" {
            case "Failure retains settings for retry and late completion is ignored" {
                verify "restore the editable request and ignore additional completion events" {
                    let mut state = ProfileSetupWindowState::editing();
                    state.update(ProfileSetupEvent::NameChanged("Work".into()).into());
                    state.update(ProfileSetupEvent::SignatureAlgorithmChanged(
                        SshSignatureAlgorithm::SkEcdsaSha2NistP256,
                    ).into());
                    state.update(ProfileSetupEvent::AdvancedSettingsToggled.into());
                    state.update(ProfileSetupEvent::PasskeyChanged(PasskeyChoice::Existing).into());
                    state.update(ProfileSetupEvent::SetupRequested.into());
                    state.update(ProfileSetupWindowEvent::Finished(std::sync::Arc::new(Err(
                        ProfileSetupFailure::Failed(ApplicationError::InvariantViolation("test failure".into())),
                    ))));
                    assert!(state.advanced_settings());
                    assert_eq!(state.signature_algorithm(), SshSignatureAlgorithm::SkEcdsaSha2NistP256);
                    assert_eq!(state.passkey(), PasskeyChoice::Existing);
                    let settled = state.clone();
                    state.update(ProfileSetupWindowEvent::Finished(std::sync::Arc::new(Err(
                        ProfileSetupFailure::Cancelled,
                    ))));
                    assert!(state == settled);
                    assert!(matches!(
                        state.update(ProfileSetupEvent::SetupRequested.into()),
                        Some(ProfileSetupWindowCommand::Start(ProfileSetupRequest {
                            device_key_signature_algorithm: SshSignatureAlgorithm::SkEcdsaSha2NistP256,
                            passkey: PasskeyChoice::Existing,
                            ..
                        }))
                    ));
                }
            }
        }
    }

    fn failed_form() -> ProfileSetupWindowState {
        let mut state = ProfileSetupWindowState::editing();
        state.update(ProfileSetupEvent::NameChanged("Work".into()).into());
        state.update(ProfileSetupEvent::SetupRequested.into());
        state.update(ProfileSetupWindowEvent::Finished(std::sync::Arc::new(Err(
            ProfileSetupFailure::Failed(ApplicationError::ProfileRepository(
                std::io::Error::other("disk unavailable"),
            )),
        ))));
        state
    }

    keyringd_testkit::scenario! {
        group "ProfileSetupWindowState::update" {
            case "Unchanged form values preserve error and allow retry" {
                verify "retain the failed form's error when the same values are submitted" {
                    let mut state = failed_form();
                    let notice = state.notice().cloned();
                    assert!(matches!(
                        notice,
                        Some(ProfileSetupNotice::OperationFailed(_))
                    ));
                    for event in [
                        ProfileSetupEvent::NameChanged("Work".into()),
                        ProfileSetupEvent::PasskeyChanged(PasskeyChoice::CreateNew),
                        ProfileSetupEvent::SignatureAlgorithmChanged(SshSignatureAlgorithm::EcdsaSha2NistP256),
                    ] {
                        state.update(event.into());
                        assert!(state.notice() == notice.as_ref());
                    }
                    assert!(matches!(
                        state.update(ProfileSetupEvent::SetupRequested.into()),
                        Some(ProfileSetupWindowCommand::Start(request))
                            if request.display_name == "Work" && request.passkey == PasskeyChoice::CreateNew
                    ));
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "ProfileSetupWindowState::update" {
            case "Actual form edits clear the previous error" {
                verify "remove the displayed setup error when a field value changes" {
                    for event in [
                        ProfileSetupEvent::NameChanged("Personal".into()),
                        ProfileSetupEvent::PasskeyChanged(PasskeyChoice::Existing),
                    ] {
                        let mut state = failed_form();
                        state.update(event.into());
                        assert!(state.notice().is_none());
                    }
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "ProfileSetupWindowState::update" {
            case "Start Setup with a valid request and issue a command" {
                given "Prepare ProfileSetupWindowState, ProfileSetupEvent, PasskeyChoice" {
                    let mut state = ProfileSetupWindowState::editing();
                    state.update(ProfileSetupEvent::NameChanged(" Work ".to_owned()).into());
                    state.update(ProfileSetupEvent::PasskeyChanged(PasskeyChoice::Existing).into());
                }
                when "Call State.Update" {
                    let command = state.update(ProfileSetupEvent::SetupRequested.into());
                }
                then "Start Setup with a valid request and issue a command" {
                    assert!(matches!(
                        command,
                        Some(ProfileSetupWindowCommand::Start(ProfileSetupRequest {
                            display_name,
                            passkey: PasskeyChoice::Existing,
                            ..
                        })) if display_name == "Work"
                    ));
                    assert!(state.is_running());
                }
            }
            case "Do not issue commands with an empty name to keep it edited" {
                given "Prepare ProfileSetupWindowState" {
                    let mut state = ProfileSetupWindowState::editing();
                }
                when "Call State.Update" {
                    let command = state.update(ProfileSetupEvent::SetupRequested.into());
                }
                then "Do not issue commands with an empty name to keep it edited" {
                    assert!(command.is_none());
                    assert!(matches!(
                        state.notice(),
                        Some(ProfileSetupNotice::MissingName)
                    ));
                }
            }
            case "Display the guide page in Wizard when Passkey is missing" {
                given "Prepare ProfileSetupWindowState, ProfileSetUpEvent" {
                    let mut state = ProfileSetupWindowState::editing();
                    state.update(ProfileSetupEvent::NameChanged("Work".to_owned()).into());
                    state.update(ProfileSetupEvent::PasskeyChanged(PasskeyChoice::Existing).into());
                    state.update(ProfileSetupEvent::SetupRequested.into());
                }
                when "Call State.Update" {
                    let command = state.update(ProfileSetupWindowEvent::Finished(std::sync::Arc::new(Err(
                        ProfileSetupFailure::PasskeyNotFound,
                    ))));
                }
                then "When Passkey is missing, go to the guide page and notify the end of the operation" {
                    assert!(matches!(
                        command,
                        Some(ProfileSetupWindowCommand::OperationSettled {
                            close_window: false,
                        })
                    ));
                    assert_eq!(
                        state.editing_page(),
                        Some(EditingPage::PasskeyNotFound(
                            PasskeyNotFoundChoice::TryAnotherDevice
                        ))
                    );
                    assert!(matches!(
                        state.notice(),
                        Some(ProfileSetupNotice::PasskeyNotFound)
                    ));
                }
            }
            case "The Key Operation display is owned by the Profile Setup Window State" {
                given "Prepare ProfileSetupWindowState, ProfileSetUpEvent" {
                    let mut state = ProfileSetupWindowState::editing();
                    state.update(ProfileSetupEvent::NameChanged("Work".to_owned()).into());
                    state.update(ProfileSetupEvent::SetupRequested.into());
                }
                when "Get a Hybrid QR Display Notification" {
                    state.update(key(KeyOperationCommand::UpdateOperation { transport: AuthenticatorTransport::hybrid() }));
                    state.update(key(KeyOperationCommand::PresentHybridQr { prompt_id: request_id(1), presented: tokio::sync::oneshot::channel().0, replace_active: false,
                            value: "FIDO:/test".to_owned(),
                            context: None,
                            unavailable: None,
                            security_key_available: true,
                        }));
                }
                then "The display of Hybrid QR and USB operation waiting is updated." {
                    assert!(
                        matches!(state.prompt(), ProfileSetupPrompt::Transports(TransportPrompts {
                                                                                hybrid: Some(HybridPrompt { value, unavailable: None, .. }),
                                                                                ..
                                                                            }) if value == "FIDO:/test")
                    );
                }
                when "Get notifications that Bluetooth is disabled" {
                    state.update(key(KeyOperationCommand::UpdateHybridAvailability { reason: keyringd_fido_client::HybridTransportUnavailableReason::BluetoothDisabled }));
                }
                then "The display of Hybrid QR and USB operation waiting is updated." {
                    assert!(matches!(
                        state.prompt(),
                        ProfileSetupPrompt::Transports(TransportPrompts {
                            hybrid: Some(HybridPrompt {
                                unavailable: Some(
                                    keyringd_fido_client::HybridTransportUnavailableReason::BluetoothDisabled
                                ),
                                ..
                            }),
                            ..
                        })
                    ));
                }
                when "Receive a USB key operation waiting notification" {
                    state.update(key(KeyOperationCommand::ShowAuthenticatorPrompt { operation: KeyOperationKind::ProfileSetup { passkey: PasskeyChoice::CreateNew },
                            transport: AuthenticatorTransport::usb(),
                            prompt_kind: AuthenticatorPromptKind::UserPresence,
                        }));
                }
                then "The display of Hybrid QR and USB operation waiting is updated." {
                    assert!(matches!(
                        state.prompt(),
                        ProfileSetupPrompt::Transports(TransportPrompts {
                            hybrid: Some(_),
                            prompted_transports,
                            ..
                        }) if prompted_transports == &[AuthenticatorTransport::usb()]
                    ));
                }
                then "After the authentication is finished, return to the ready display for the next process." {
                    state.update(key(KeyOperationCommand::CloseOperationStatus));
                    assert!(matches!(state.prompt(), ProfileSetupPrompt::Preparing));
                }
            }
        }
    }

    fn request_id(sequence: u64) -> keyringd_fido_client::InteractionId {
        keyringd_fido_client::InteractionId::new(OperationId::new(1), sequence)
    }

    fn running() -> ProfileSetupWindowState {
        let mut state = ProfileSetupWindowState::default();
        state.update(ProfileSetupEvent::NameChanged("Work".into()).into());
        assert!(matches!(
            state.update(ProfileSetupEvent::SetupRequested.into()),
            Some(ProfileSetupWindowCommand::Start(_))
        ));
        assert!(
            state
                .update(ProfileSetupEvent::SetupRequested.into())
                .is_none()
        );
        state
    }
    fn finished() -> ProfileSetupWindowEvent {
        ProfileSetupWindowEvent::Finished(Arc::new(Err(ProfileSetupFailure::Cancelled)))
    }
    fn pin(request_id: keyringd_fido_client::InteractionId) -> ProfileSetupWindowEvent {
        ProfileSetupWindowEvent::PinRequested {
            request_id,
            prompt: SecurityKeyPinPrompt {
                reason: "Enter PIN".into(),
                attempts_left: Some(3),
                minimum_length: Some(4),
                new_pin: false,
            },
        }
    }
    fn submit(request_id: keyringd_fido_client::InteractionId) -> ProfileSetupWindowEvent {
        ProfileSetupEvent::PinCompleted {
            request_id,
            command: PinPromptCommand::Submit("1234".into()),
        }
        .into()
    }
    fn key(command: KeyOperationCommand) -> ProfileSetupWindowEvent {
        ProfileSetupWindowEvent::KeyOperation(command)
    }

    keyringd_testkit::scenario! {
        group "ProfileSetupWindowState::update" {
            case "Close waits for cleanup and does not leak into the next setup" {
                verify "defer closing until cleanup completes and clear the close intent before reuse" {
                    let mut state = running();
                    assert!(matches!(
                        state.update(ProfileSetupEvent::Dismissed.into()),
                        Some(ProfileSetupWindowCommand::CancelOperation)
                    ));
                    assert!(state.is_closing());
                    assert!(state.update(ProfileSetupEvent::Dismissed.into()).is_none());
                    state.update(pin(request_id(1)));
                    assert!(matches!(
                        state.update(key(KeyOperationCommand::CloseOperationStatus)),
                        Some(ProfileSetupWindowCommand::KeyOperation(
                            KeyOperationCommand::CloseOperationStatus
                        ))
                    ));
                    assert!(matches!(
                        state.interaction_presentation().screen,
                        ProfileSetupInteractionScreen::Cancelling
                    ));
                    assert!(state.update(submit(request_id(1))).is_none());
                    assert!(matches!(
                        state.update(finished()),
                        Some(ProfileSetupWindowCommand::OperationSettled { close_window: true })
                    ));
                    assert!(state.update(finished()).is_none());
                    assert!(!state.is_closing());
                    state.update(ProfileSetupEvent::SetupRequested.into());
                    assert!(matches!(
                        state.update(finished()),
                        Some(ProfileSetupWindowCommand::OperationSettled {
                            close_window: false
                        })
                    ));
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "ProfileSetupWindowState::update" {
            case "Back cancels without closing and retains settings for retry" {
                verify "keep the window and its form data after a Back cancellation" {
                    let mut state = running();
                    assert!(state.can_go_back());
                    assert!(matches!(
                        state.update(ProfileSetupEvent::BackRequested.into()),
                        Some(ProfileSetupWindowCommand::CancelOperation)
                    ));
                    assert!(state.update(ProfileSetupEvent::BackRequested.into()).is_none());
                    assert!(!state.can_go_back());
                    assert!(state.update(ProfileSetupEvent::Dismissed.into()).is_none());
                    assert!(!state.is_closing());
                    state.update(pin(request_id(1)));
                    assert!(state.update(submit(request_id(1))).is_none());
                    assert!(matches!(
                        state.update(finished()),
                        Some(ProfileSetupWindowCommand::OperationSettled {
                            close_window: false
                        })
                    ));
                    assert_eq!(state.name(), "Work");
                    assert!(state.can_start());
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "ProfileSetupWindowState::update" {
            case "Stale and duplicate pin completions cannot answer the current request" {
                verify "accept only the current outstanding PIN ID and consume it on completion" {
                    let mut state = running();
                    state.update(pin(request_id(1)));
                    state.update(pin(request_id(2)));
                    assert!(state.update(submit(request_id(1))).is_none());
                    assert_eq!(state.pin_request_id(), Some(request_id(2)));
                    assert!(matches!(
                        state.update(submit(request_id(2))),
                        Some(ProfileSetupWindowCommand::RespondPin { request_id: actual, .. }) if actual == request_id(2)
                    ));
                    assert!(state.update(submit(request_id(2))).is_none());
                    assert!(!matches!(
                        state.interaction_presentation().screen,
                        ProfileSetupInteractionScreen::Pin(_)
                    ));
                    state.update(finished());
                    state.update(pin(request_id(3)));
                    assert!(state.pin_request_id().is_none());
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "ProfileSetupWindowState::update" {
            case "Recovery certificate QR uses progress when public-key confirmation is skipped" {
                verify "the second QR describes certificate authorization rather than verification" {
                    let mut state = running();
                    if let ProfileSetupWindowState::Running { passkey, .. } = &mut state {
                        *passkey = PasskeyChoice::Existing;
                    }
                    state.update(ProfileSetupWindowEvent::Progressed(
                        ProfileSetupProgress::ProvisioningDeviceKey,
                    ));
                    state.update(key(KeyOperationCommand::PresentHybridQr {
                        prompt_id: request_id(2),
                        value: "FIDO:/certificate".into(),
                        context: None,
                        security_key_available: false,
                        presented: tokio::sync::oneshot::channel().0,
                        replace_active: false,
                        unavailable: None,
                    }));
                    assert_eq!(state.interaction_presentation().context,
                        Some(AuthenticatorPromptPurpose::AuthorizeWithExistingPasskey));
                    assert_eq!(state.interaction_presentation().stage,
                        Some(crate::profile_setup_window::ProfileSetupStep::AuthorizeExisting));
                }
            }
            case "Qr acknowledgement follows the shared presentation and keeps pin visible" {
                async: tokio;
                timeout: std::time::Duration::from_secs(60);
                verify "acknowledge the QR dismissal while retaining the active PIN presentation" {
                    let mut state = running();
                    let (presented, shown) = tokio::sync::oneshot::channel();
                    let command = state.update(key(KeyOperationCommand::PresentHybridQr {
                        prompt_id: request_id(1),
                        value: "FIDO:/test".into(),
                        context: None,
                        security_key_available: true,
                        presented,
                        replace_active: false,
                        unavailable: None,
                    }));
                    assert!(matches!(
                        state.interaction_presentation().screen,
                        ProfileSetupInteractionScreen::Qr(_)
                    ));
                    let Some(ProfileSetupWindowCommand::Acknowledge(presented)) = command else {
                        panic!("QR acknowledgement");
                    };
                    presented.send(()).unwrap();
                    shown.await.unwrap();
                    assert!(
                        state
                            .update(ProfileSetupEvent::UseSecurityKeyRequested(OperationId::new(
                                9
                            )).into())
                            .is_none()
                    );
                    state.update(pin(request_id(1)));
                    state.update(key(KeyOperationCommand::UpdateHybridAvailability {
                        reason: HybridTransportUnavailableReason::BluetoothDisabled,
                    }));
                    assert!(matches!(
                        state.interaction_presentation().screen,
                        ProfileSetupInteractionScreen::Pin(_)
                    ));
                    let (acknowledged, closed) = tokio::sync::oneshot::channel();
                    let Some(ProfileSetupWindowCommand::Acknowledge(acknowledged)) =
                        state.update(key(KeyOperationCommand::DismissHybridQr {
                            prompt_id: request_id(1),
                            acknowledged,
                        }))
                    else {
                        panic!("dismissal acknowledgement");
                    };
                    acknowledged.send(()).unwrap();
                    closed.await.unwrap();
                    assert!(matches!(
                        state.interaction_presentation().screen,
                        ProfileSetupInteractionScreen::Pin(_)
                    ));
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "ProfileSetupWindowState::update" {
            case "Usb selection is checked against the presented operation before emitting a command" {
                verify "dispatch a USB switch only for the operation ID from the current QR" {
                    let mut state = running();
                    let (presented, _) = tokio::sync::oneshot::channel();
                    state.update(key(KeyOperationCommand::PresentHybridQr {
                        prompt_id: request_id(1),
                        value: "FIDO:/test".into(),
                        context: None,
                        security_key_available: true,
                        presented,
                        replace_active: false,
                        unavailable: None,
                    }));
                    assert!(
                        state
                            .update(ProfileSetupEvent::UseSecurityKeyRequested(OperationId::new(2)).into())
                            .is_none()
                    );
                    assert!(matches!(
                        state.update(ProfileSetupEvent::UseSecurityKeyRequested(OperationId::new(1)).into()),
                        Some(ProfileSetupWindowCommand::UseUsb(operation)) if operation == OperationId::new(1)
                    ));
                    assert!(
                        state
                            .update(ProfileSetupEvent::UseSecurityKeyRequested(OperationId::new(1)).into())
                            .is_none()
                    );
                    state.update(ProfileSetupWindowEvent::Progressed(ProfileSetupProgress::ProvisioningDeviceKey));
                    assert!(
                        state
                            .update(ProfileSetupEvent::UseSecurityKeyRequested(OperationId::new(1)).into())
                            .is_none()
                    );
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "ProfileSetupWindowState::update" {
            case "Old qr usb selection cannot replace a pin prompt in the same operation" {
                verify "ignore a USB switch from a QR that was replaced by PIN entry" {
                    let mut state = running();
                    let (presented, _) = tokio::sync::oneshot::channel();
                    state.update(key(KeyOperationCommand::PresentHybridQr {
                        prompt_id: request_id(1),
                        value: "FIDO:/test".into(),
                        context: None,
                        security_key_available: true,
                        presented,
                        replace_active: false,
                        unavailable: None,
                    }));
                    state.update(pin(request_id(2)));
                    assert!(
                        state
                            .update(ProfileSetupEvent::UseSecurityKeyRequested(OperationId::new(1)).into())
                            .is_none()
                    );
                    assert_eq!(state.pin_request_id(), Some(request_id(2)));
                    assert!(matches!(
                        state.interaction_presentation().screen,
                        ProfileSetupInteractionScreen::Pin(_)
                    ));
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "ProfileSetupWindowState::update" {
            case "Progress and completion clear old authenticator input" {
                verify "invalidate pending authenticator input when the step advances or completes" {
                    let mut state = running();
                    state.update(pin(request_id(1)));
                    state.update(ProfileSetupWindowEvent::Progressed(ProfileSetupProgress::ProvisioningDeviceKey));
                    assert!(state.pin_request_id().is_none());
                    assert!(state.interaction_presentation().operation_id.is_none());
                    assert!(state.update(submit(request_id(1))).is_none());
                    state.update(finished());
                    state.update(key(KeyOperationCommand::UpdateHybridAvailability {
                        reason: HybridTransportUnavailableReason::BluetoothDisabled,
                    }));
                    assert!(!state.is_running());
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "ProfileSetupWindowState::update" {
            case "Missing passkey retry restarts recovery and new passkey choice returns to the form" {
                verify "restart recovery directly on Retry and return to editing on Create New" {
                    let mut state = ProfileSetupWindowState::default();
                    state.update(ProfileSetupEvent::NameChanged("Work".into()).into());
                    state.update(ProfileSetupEvent::PasskeyChanged(PasskeyChoice::Existing).into());
                    state.update(ProfileSetupEvent::SignatureAlgorithmChanged(SshSignatureAlgorithm::SkEcdsaSha2NistP256).into());
                    state.update(ProfileSetupEvent::SetupRequested.into());
                    let missing = || {
                        ProfileSetupWindowEvent::Finished(Arc::new(Err(ProfileSetupFailure::PasskeyNotFound)))
                    };
                    state.update(missing());
                    assert!(matches!(
                        state.update(ProfileSetupEvent::PasskeyNotFoundChoiceConfirmed.into()),
                        Some(ProfileSetupWindowCommand::Start(ProfileSetupRequest {
                            passkey: PasskeyChoice::Existing,
                            device_key_signature_algorithm: SshSignatureAlgorithm::SkEcdsaSha2NistP256,
                            ..
                        }))
                    ));
                    state.update(missing());
                    state.update(ProfileSetupEvent::PasskeyNotFoundChoiceChanged(
                            crate::profile_setup_window::PasskeyNotFoundChoice::CreateNewPasskey,
                        )
                        .into());
                    assert!(
                        state
                            .update(ProfileSetupEvent::PasskeyNotFoundChoiceConfirmed.into())
                            .is_none()
                    );
                    assert_eq!(state.passkey(), PasskeyChoice::CreateNew);
                    assert!(matches!(
                        state.update(ProfileSetupEvent::SetupRequested.into()),
                        Some(ProfileSetupWindowCommand::Start(ProfileSetupRequest {
                            passkey: PasskeyChoice::CreateNew,
                            ..
                        }))
                    ));
                }
            }
        }
    }
}
