use keyringd_core::application::PasskeyChoice;
use keyringd_ssh_agent_protocol::SshSignatureAlgorithm;
use keyringd_ui_core::text as ui_text;

use keyringd_ui_core::runtime::UiEventTarget;

use windows_reactor::{Element, LayoutExt, ThemeRef, border, text_block};

use keyringd_ui_core::profile_setup_window::ProfileSetupEvent as FormEvent;
use keyringd_ui_core::profile_setup_window::ProfileSetupWindowState;

use windows_reactor::{
    AccessibilityExt, ElementRef, ElementRefExt, Expander, InfoBar, InfoBarSeverity, RadioButtons,
    TextBoxHandle, TextStyleExt, text_box, vstack,
};

pub(crate) struct ProfileFormPage;

impl ProfileFormPage {
    pub(crate) fn render(
        state: &ProfileSetupWindowState,
        name_input: ElementRef<TextBoxHandle>,
        on_event: UiEventTarget<FormEvent>,
    ) -> Element {
        let advanced_settings = state.advanced_settings();
        let name_event = on_event.clone();
        let format_event = on_event.clone();
        let expanded_event = on_event.clone();
        let choice_event = on_event.clone();
        let cancelled: Element = if state.cancelled() {
            text_block(ui_text::PROFILE_SETUP_CANCELLED)
                .automation_id("profile-setup-cancelled")
                .wrap()
                .into()
        } else {
            border(text_block("")).height(0.0).into()
        };
        let error: Element = state
            .error()
            .map(|message| {
                InfoBar::new(message)
                    .automation_id("profile-setup-error")
                    .automation_name(message)
                    .severity(InfoBarSeverity::Error)
                    .is_open(true)
                    .into()
            })
            .unwrap_or_else(|| border(text_block("")).height(0.0).into());
        vstack((
            text_block(ui_text::PROFILE_SETUP_FORM_DESCRIPTION)
                .wrap()
                .foreground(ThemeRef::SecondaryText),
            text_box(state.name())
                .automation_id("profile-setup-name")
                .element_ref(&name_input)
                .header(ui_text::PROFILE_SETUP_NAME)
                .on_text_changed(move |name| name_event.send(FormEvent::NameChanged(name))),
            text_block(ui_text::PROFILE_SETUP_PASSKEY).semibold(),
            RadioButtons::new([
                ui_text::PROFILE_SETUP_CREATE_CHOICE,
                ui_text::PROFILE_SETUP_EXISTING_CHOICE,
            ])
            .automation_id("profile-setup-passkey-choice")
            .selected_index(if state.passkey() == PasskeyChoice::CreateNew {
                0
            } else {
                1
            })
            .on_selection_changed(move |index| {
                choice_event.send(FormEvent::PasskeyChanged(if index == 0 {
                    PasskeyChoice::CreateNew
                } else {
                    PasskeyChoice::Existing
                }));
            }),
            Expander::new(
                vstack((
                    text_block(ui_text::PROFILE_SETUP_SIGNATURE_FORMAT).semibold(),
                    text_block(ui_text::PROFILE_SETUP_SIGNATURE_DESCRIPTION),
                    RadioButtons::new([
                        ui_text::PROFILE_SETUP_ECDSA_CHOICE,
                        ui_text::PROFILE_SETUP_SECURITY_KEY_CHOICE,
                    ])
                    .automation_id("profile-setup-signature-format")
                    .selected_index(
                        if state.signature_algorithm() == SshSignatureAlgorithm::EcdsaSha2NistP256 {
                            0
                        } else {
                            1
                        },
                    )
                    .on_selection_changed(move |index| {
                        format_event.send(FormEvent::SignatureAlgorithmChanged(if index == 0 {
                            SshSignatureAlgorithm::EcdsaSha2NistP256
                        } else {
                            SshSignatureAlgorithm::SkEcdsaSha2NistP256
                        }))
                    }),
                ))
                .spacing(10.0),
            )
            .header(ui_text::PROFILE_SETUP_ADVANCED)
            .automation_id("profile-setup-advanced-settings")
            .expanded(advanced_settings)
            .on_expanding(move |expanded| {
                if expanded != advanced_settings {
                    expanded_event.send(FormEvent::AdvancedSettingsToggled);
                }
            }),
            cancelled,
            error,
        ))
        .spacing(14.0)
        .into()
    }
}
