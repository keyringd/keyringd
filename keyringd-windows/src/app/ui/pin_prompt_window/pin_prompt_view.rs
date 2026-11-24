use keyringd_ui_core::pin_prompt_window::{
    PinPromptCommand, PinPromptEvent, PinPromptState, SecurityKeyPinPrompt,
};
use keyringd_ui_core::runtime::UiEventTarget;
use keyringd_ui_core::text as ui_text;
use windows_reactor::{
    BackgroundExt, Element, GridChildExt, GridLength, HorizontalAlignment, InputExt, KeyExt,
    KeyboardAccelerator, LayoutExt, PaddingExt, PasswordBox, RenderCx, TextStyleExt, ThemeRef,
    Thickness, Updater, VirtualKey, VirtualKeyModifiers, border, button, component, grid, hstack,
    scroll_viewer, text_block, vstack,
};

/// Owns PIN input and validation independently of its containing native window.
pub(in crate::app::ui) struct PinPromptView;

impl PinPromptView {
    pub(in crate::app::ui) fn render(
        kind: &SecurityKeyPinPrompt,
        request_id: keyringd_fido_client::InteractionId,
        dismissed: bool,
        completion: UiEventTarget<PinPromptCommand>,
    ) -> Element {
        component(
            move |props: &(SecurityKeyPinPrompt, bool), cx: &mut RenderCx| {
                Self::render_content(cx, &props.0, props.1, completion.clone())
            },
            (kind.clone(), dismissed),
        )
        .with_key(format!("pin-input-{request_id:?}"))
    }
    fn render_content(
        cx: &mut RenderCx,
        kind: &SecurityKeyPinPrompt,
        dismissed: bool,
        completion: UiEventTarget<PinPromptCommand>,
    ) -> Element {
        let (state, set_state) = cx.use_reducer(PinPromptState::for_prompt(kind));
        cx.use_effect(dismissed, {
            let state = set_state.clone();
            let completion = completion.clone();
            move || {
                if dismissed {
                    Self::dispatch(&state, &completion, PinPromptEvent::Dismissed);
                }
            }
        });
        let pin_set_state = set_state.clone();
        let pin_completion = completion.clone();
        let confirmation_set_state = set_state.clone();
        let confirmation_completion = completion.clone();
        let submit_set_state = set_state.clone();
        let submit_completion = completion.clone();
        let cancel_set_state = set_state.clone();
        let cancel_completion = completion.clone();
        let enter_set_state = set_state.clone();
        let enter_completion = completion.clone();
        let escape_set_state = set_state;
        let escape_completion = completion;
        let heading = keyringd_ui_core::pin_prompt_window::pin_heading(kind.new_pin);
        let explanation = keyringd_ui_core::pin_prompt_window::pin_explanation(
            &kind.reason,
            kind.minimum_length,
            kind.attempts_left,
        );
        let mut content: Vec<Element> = vec![
            text_block(heading).font_size(24.0).semibold().into(),
            text_block(explanation)
                .wrap()
                .foreground(ThemeRef::SecondaryText)
                .into(),
            PasswordBox::new()
                .value(state.pin())
                .header(ui_text::PIN_PROMPT_PIN_LABEL)
                .placeholder_text(ui_text::PIN_PROMPT_PIN_LABEL)
                .on_password_changed(move |pin| {
                    Self::dispatch(
                        &pin_set_state,
                        &pin_completion,
                        PinPromptEvent::PinChanged(pin),
                    )
                })
                .into(),
        ];
        if state.requires_confirmation() {
            content.push(
                PasswordBox::new()
                    .value(state.confirmation())
                    .header(ui_text::PIN_PROMPT_CONFIRM_PIN_LABEL)
                    .placeholder_text(ui_text::PIN_PROMPT_CONFIRM_PIN_LABEL)
                    .on_password_changed(move |confirmation| {
                        Self::dispatch(
                            &confirmation_set_state,
                            &confirmation_completion,
                            PinPromptEvent::ConfirmationChanged(confirmation),
                        )
                    })
                    .into(),
            );
        }
        if let Some(message) = state.validation_message() {
            content.push(
                text_block(message)
                    .wrap()
                    .foreground(ThemeRef::SecondaryText)
                    .into(),
            );
        }
        let body = scroll_viewer(
            vstack(content)
                .spacing(16.0)
                .padding(Thickness::uniform(24.0)),
        )
        .grid_row(0);
        let footer = border(
            hstack((
                button(ui_text::CONTINUE)
                    .accent()
                    .min_width(88.0)
                    .enabled(state.can_submit())
                    .on_click(move || {
                        Self::dispatch(
                            &submit_set_state,
                            &submit_completion,
                            PinPromptEvent::SubmitRequested,
                        )
                    }),
                button(ui_text::CANCEL).min_width(88.0).on_click(move || {
                    Self::dispatch(
                        &cancel_set_state,
                        &cancel_completion,
                        PinPromptEvent::CancelRequested,
                    )
                }),
            ))
            .spacing(8.0)
            .horizontal_alignment(HorizontalAlignment::Right),
        )
        .padding(Thickness::xy(24.0, 16.0))
        .background(ThemeRef::LayerFill)
        .border_brush(ThemeRef::CardStroke)
        .border_thickness(Thickness {
            left: 0.0,
            top: 1.0,
            right: 0.0,
            bottom: 0.0,
        })
        .grid_row(1);
        grid((body, footer))
            .rows([GridLength::STAR, GridLength::Auto])
            .keyboard_accelerator(KeyboardAccelerator::new(
                VirtualKey::Enter,
                VirtualKeyModifiers::None,
                move || {
                    Self::dispatch(
                        &enter_set_state,
                        &enter_completion,
                        PinPromptEvent::SubmitRequested,
                    )
                },
            ))
            .keyboard_accelerator(KeyboardAccelerator::new(
                VirtualKey::Escape,
                VirtualKeyModifiers::None,
                move || {
                    Self::dispatch(
                        &escape_set_state,
                        &escape_completion,
                        PinPromptEvent::CancelRequested,
                    )
                },
            ))
            .into()
    }

    pub(super) fn dispatch(
        set_state: &Updater<PinPromptState>,
        completion: &UiEventTarget<PinPromptCommand>,
        event: PinPromptEvent,
    ) {
        let (_, command) = crate::app::ui::reduce(set_state, event);
        if let Some(command) = command {
            completion.send(command);
        }
    }
}
