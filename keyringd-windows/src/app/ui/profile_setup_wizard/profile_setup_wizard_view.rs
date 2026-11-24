use crate::app::ui::profile_setup_interaction::ProfileSetupInteractionPage;
use crate::app::ui::profile_setup_wizard::{CompletionPage, ProfileFormPage};
use keyringd_ui_core::profile_setup_window::ProfileSetupEvent;
use keyringd_ui_core::profile_setup_window::ProfileSetupWindowState;

use keyringd_ui_core::runtime::UiEventTarget;
use keyringd_ui_core::text as ui_text;
use windows_reactor::{
    AccessibilityExt, BackgroundExt, DispatcherTimer, Element, ElementRef, GridChildExt,
    GridLength, HorizontalAlignment, InputExt, KeyExt, KeyboardAccelerator, LayoutExt, PaddingExt,
    RenderCx, TextBoxHandle, ThemeRef, Thickness, VirtualKey, VirtualKeyModifiers, border, button,
    grid, hstack, scroll_viewer, text_block,
};

pub(crate) struct ProfileSetupWizardView;
impl ProfileSetupWizardView {
    pub(crate) fn render(
        cx: &mut RenderCx,
        state: &ProfileSetupWindowState,
        interaction: &keyringd_ui_core::profile_setup_window::ProfileSetupInteractionPresentation,
        pin_request_id: Option<keyringd_fido_client::InteractionId>,
        events: UiEventTarget<ProfileSetupEvent>,
    ) -> Element {
        let input = cx.use_element_ref::<TextBoxHandle>();
        Self::focus_name_input(cx, input.clone(), state.is_editing());
        let primary_event = if state.credential_selection().is_some() {
            ProfileSetupEvent::CredentialSelectionConfirmed
        } else if state.completed_id().is_some() {
            ProfileSetupEvent::Dismissed
        } else if matches!(
            state.editing_page(),
            Some(keyringd_ui_core::profile_setup_window::EditingPage::PasskeyNotFound(_))
        ) {
            ProfileSetupEvent::PasskeyNotFoundChoiceConfirmed
        } else {
            ProfileSetupEvent::SetupRequested
        };
        let enter_events = events.clone();
        let escape_events = events.clone();
        let content = if let Some(id) = state.completed_id().cloned() {
            let trust_events = enter_events.clone();

            let done_events = enter_events.clone();

            Self::layout(
                ui_text::PROFILE_SETUP_COMPLETED_TITLE,
                CompletionPage::render(
                    keyringd_ui_core::profile_setup_window::completed_description(
                        state
                            .completed_profile()
                            .expect("completed setup has a profile")
                            .display_name(),
                    ),
                    state.notice_message().map(str::to_owned),
                ),
                hstack((
                    button(ui_text::TRUST_ENTRIES_MENU_ACTION)
                        .automation_id("profile-setup-public-keys")
                        .with_key("setup-trust")
                        .on_click(move || {
                            trust_events.send(ProfileSetupEvent::ShowTrustEntries);
                        }),
                    button(ui_text::DONE)
                        .automation_id("profile-setup-done")
                        .with_key(format!("setup-done-{}", id.as_str()))
                        .accent()
                        .on_click(move || {
                            done_events.send(ProfileSetupEvent::Dismissed);
                        }),
                ))
                .spacing(8.0)
                .horizontal_alignment(HorizontalAlignment::Right)
                .into(),
            )
        } else if let Some(selection) = state.credential_selection() {
            let continue_events = enter_events.clone();

            let cancel_events = enter_events.clone();

            Self::layout(
                ui_text::PROFILE_SETUP_TITLE,
                crate::app::ui::profile_setup_wizard::PasskeySelectionPage::render(
                    selection.clone(),
                    events.clone(),
                ),
                hstack((
                    {
                        let back_events = events.clone();
                        button(ui_text::BACK)
                            .on_click(move || back_events.send(ProfileSetupEvent::BackRequested))
                    },
                    button(ui_text::CONTINUE)
                        .automation_id("profile-setup-select-passkey")
                        .accent()
                        .enabled(selection.selected_id().is_some())
                        .on_click(move || {
                            continue_events.send(ProfileSetupEvent::CredentialSelectionConfirmed)
                        }),
                    button(ui_text::CANCEL)
                        .automation_id("profile-setup-cancel")
                        .on_click(move || cancel_events.send(ProfileSetupEvent::Dismissed)),
                ))
                .spacing(8.0)
                .horizontal_alignment(HorizontalAlignment::Right)
                .into(),
            )
        } else if let Some(keyringd_ui_core::profile_setup_window::EditingPage::PasskeyNotFound(
            choice,
        )) = state.editing_page()
        {
            use keyringd_ui_core::profile_setup_window::PasskeyNotFoundChoice;
            use windows_reactor::{RadioButtons, vstack};
            let choice_events = enter_events.clone();

            let next_events = enter_events.clone();

            let back_events = enter_events.clone();

            let cancel_events = enter_events.clone();

            Self::layout(
                ui_text::PROFILE_SETUP_NOT_FOUND_TITLE,
                vstack((
                    text_block(ui_text::PROFILE_SETUP_NOT_FOUND)
                        .automation_id("profile-setup-not-found")
                        .wrap(),
                    text_block(ui_text::PROFILE_SETUP_RECOVERY_WARNING).wrap(),
                    RadioButtons::new([
                        ui_text::PROFILE_SETUP_TRY_ANOTHER_DEVICE,
                        ui_text::PROFILE_SETUP_CREATE_INSTEAD,
                    ])
                    .automation_id("profile-setup-not-found-choice")
                    .selected_index(if choice == PasskeyNotFoundChoice::TryAnotherDevice {
                        0
                    } else {
                        1
                    })
                    .on_selection_changed(move |index| {
                        choice_events.send(ProfileSetupEvent::PasskeyNotFoundChoiceChanged(
                            if index == 0 {
                                PasskeyNotFoundChoice::TryAnotherDevice
                            } else {
                                PasskeyNotFoundChoice::CreateNewPasskey
                            },
                        ))
                    }),
                ))
                .spacing(14.0)
                .into(),
                hstack((
                    button(ui_text::BACK)
                        .on_click(move || back_events.send(ProfileSetupEvent::BackToSetup)),
                    button(ui_text::CONTINUE)
                        .automation_id("profile-setup-not-found-continue")
                        .accent()
                        .on_click(move || {
                            next_events.send(ProfileSetupEvent::PasskeyNotFoundChoiceConfirmed)
                        }),
                    button(ui_text::CANCEL)
                        .automation_id("profile-setup-cancel")
                        .on_click(move || cancel_events.send(ProfileSetupEvent::Dismissed)),
                ))
                .spacing(8.0)
                .horizontal_alignment(HorizontalAlignment::Right)
                .into(),
            )
        } else if state.is_running() {
            let cancel_events = events.clone();
            let back_events = events.clone();
            let body =
                ProfileSetupInteractionPage::render(interaction, pin_request_id, events.clone());
            Self::layout(
                ui_text::PROFILE_SETUP_TITLE,
                body,
                if state.can_go_back() {
                    hstack((
                        button(ui_text::BACK)
                            .on_click(move || back_events.send(ProfileSetupEvent::BackRequested)),
                        button(ui_text::CANCEL)
                            .automation_id("profile-setup-cancel")
                            .with_key("setup-cancel")
                            .on_click(move || {
                                cancel_events.send(ProfileSetupEvent::Dismissed);
                            }),
                    ))
                    .spacing(8.0)
                    .horizontal_alignment(HorizontalAlignment::Right)
                    .into()
                } else {
                    Element::Empty
                },
            )
        } else {
            let on_event = events.clone();
            let create_events = enter_events.clone();

            let cancel_events = enter_events.clone();

            let body = ProfileFormPage::render(state, input, on_event);
            Self::layout(
                ui_text::PROFILE_SETUP_TITLE,
                body,
                hstack((
                    button(state.submit_label())
                        .automation_id("profile-setup-create")
                        .with_key("setup-create")
                        .accent()
                        .enabled(state.can_start())
                        .on_click(move || {
                            create_events.send(ProfileSetupEvent::SetupRequested);
                        }),
                    button(ui_text::CANCEL)
                        .automation_id("profile-setup-cancel")
                        .with_key("setup-cancel")
                        .on_click(move || {
                            cancel_events.send(ProfileSetupEvent::Dismissed);
                        }),
                ))
                .spacing(8.0)
                .horizontal_alignment(HorizontalAlignment::Right)
                .into(),
            )
        };

        grid((content,))
            .keyboard_accelerator(KeyboardAccelerator::new(
                VirtualKey::Enter,
                VirtualKeyModifiers::None,
                move || {
                    enter_events.send(primary_event.clone());
                },
            ))
            .keyboard_accelerator(KeyboardAccelerator::new(
                VirtualKey::Escape,
                VirtualKeyModifiers::None,
                move || {
                    escape_events.send(ProfileSetupEvent::Dismissed);
                },
            ))
            .into()
    }

    fn focus_name_input(cx: &mut RenderCx, input: ElementRef<TextBoxHandle>, editing: bool) {
        cx.use_effect_with_cleanup(editing, move || {
            let timer = editing
                .then(|| {
                    DispatcherTimer::new_one_shot(std::time::Duration::from_millis(1), move || {
                        let _ = input.focus();
                    })
                    .ok()
                })
                .flatten();
            Some(move || drop(timer))
        });
    }

    fn layout(heading: &str, content: Element, footer: Element) -> Element {
        grid((
            border(
                text_block(heading)
                    .automation_id("profile-setup-title")
                    .font_size(24.0)
                    .semibold(),
            )
            .padding(Thickness::xy(24.0, 20.0))
            .grid_row(0),
            scroll_viewer(border(content).padding(Thickness::xy(24.0, 12.0))).grid_row(1),
            border(footer)
                .padding(Thickness::uniform(16.0))
                .background(ThemeRef::LayerFill)
                .border_brush(ThemeRef::CardStroke)
                .border_thickness(Thickness {
                    left: 0.0,
                    top: 1.0,
                    right: 0.0,
                    bottom: 0.0,
                })
                .grid_row(2),
        ))
        .rows([GridLength::Auto, GridLength::STAR, GridLength::Auto])
        .into()
    }
}
