use crate::app::ui::qr_prompt_window::PasskeyInteractionView;
use keyringd_ui_core::key_operation::KeyOperationEvent;
use keyringd_ui_core::profile_setup_window::ProfileSetupEvent;
use keyringd_ui_core::profile_setup_window::{
    ProfileSetupInteractionPresentation, ProfileSetupInteractionScreen,
};
use keyringd_ui_core::qr_prompt_window::HybridQrPresentation;
use keyringd_ui_core::runtime::UiEventTarget;
use windows_reactor::{AccessibilityExt, Element, text_block, vstack};

pub(crate) struct ProfileSetupInteractionPage;
impl ProfileSetupInteractionPage {
    pub(crate) fn render(
        interaction: &ProfileSetupInteractionPresentation,
        pin_request_id: Option<keyringd_fido_client::InteractionId>,
        on_event: UiEventTarget<ProfileSetupEvent>,
    ) -> Element {
        if matches!(interaction.screen, ProfileSetupInteractionScreen::Hidden) {
            return Element::Empty;
        }
        let cancelling = matches!(
            interaction.screen,
            ProfileSetupInteractionScreen::Cancelling
        );
        let heading = text_block(if cancelling {
            interaction.title()
        } else {
            interaction.stage_title()
        })
        .semibold()
        .wrap();
        let heading = heading.automation_id("profile-setup-progress-heading");
        let content = match &interaction.screen {
            ProfileSetupInteractionScreen::Pin(_) => {
                let request_id = pin_request_id.expect(
                    "an embedded PIN presentation must retain its response request identity",
                );
                crate::app::ui::pin_prompt_window::PinPromptView::render(
                    &interaction
                        .pin_prompt()
                        .expect("PIN screen carries a PIN prompt"),
                    request_id,
                    false,
                    on_event.map(move |command| ProfileSetupEvent::PinCompleted {
                        request_id,
                        command,
                    }),
                )
            }
            ProfileSetupInteractionScreen::Qr(prompt) => {
                let presentation = HybridQrPresentation::new(
                    &prompt.value,
                    &prompt.context,
                    prompt.security_key_available,
                    prompt.unavailable,
                );
                let operation_id = interaction
                    .operation_id
                    .expect("QR presentation belongs to an active FIDO operation");
                PasskeyInteractionView::render(
                    &presentation,
                    operation_id,
                    UiEventTarget::new(move |event| match event {
                        KeyOperationEvent::UseSecurityKeyRequested => {
                            on_event.send(ProfileSetupEvent::UseSecurityKeyRequested(operation_id));
                        }
                        event => event.reject(),
                    }),
                )
            }
            ProfileSetupInteractionScreen::Waiting { .. } => vstack((
                text_block(interaction.title()).semibold().wrap(),
                text_block(interaction.description()).wrap(),
            ))
            .spacing(12.0)
            .into(),
            ProfileSetupInteractionScreen::Preparing
            | ProfileSetupInteractionScreen::Cancelling
            | ProfileSetupInteractionScreen::Hidden => Element::Empty,
        };
        vstack((
            heading,
            text_block(if cancelling {
                interaction.description()
            } else {
                interaction.stage_description()
            })
            .wrap(),
            content,
        ))
        .spacing(12.0)
        .into()
    }
}
