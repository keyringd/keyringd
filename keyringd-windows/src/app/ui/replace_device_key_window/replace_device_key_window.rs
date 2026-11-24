use keyringd_ui_core::key_operation::KeyOperationCoordinator;
use keyringd_ui_core::replace_device_key_window::{
    DeviceKeyReplacementOutcome, ReplaceDeviceKeyEvent, ReplaceDeviceKeyState,
};
use keyringd_ui_core::text as ui_text;
use std::sync::Arc;
use windows_reactor::MutationState;

use crate::app::WindowsApplication;

use crate::app::ui::util::AsyncMutation;

use keyringd_core::model::ProfileId;

use windows_reactor::{
    BackgroundExt, Element, GridChildExt, GridLength, HorizontalAlignment, InputExt,
    KeyboardAccelerator, LayoutExt, PaddingExt, RenderCx, ThemeRef, Thickness, Updater, VirtualKey,
    VirtualKeyModifiers, border, button, grid, hstack, scroll_viewer, text_block, vstack,
};

use crate::app::ui::AuxiliaryWindow;

use crate::app::ui::replace_device_key_window::ReplaceDeviceKeyWindowCommandHandler;

pub(crate) struct ReplaceDeviceKeyWindow;

impl ReplaceDeviceKeyWindow {
    pub(crate) fn open(
        profile_application: Arc<WindowsApplication>,
        coordinator: Arc<KeyOperationCoordinator>,
        runtime: tokio::runtime::Handle,
        id: ProfileId,
        parent: usize,
        on_toast: keyringd_ui_core::runtime::UiEventTarget<keyringd_ui_core::toast::ToastMessage>,
    ) -> anyhow::Result<()> {
        AuxiliaryWindow::open_modal(
            ui_text::REPLACE_DEVICE_KEY_WINDOW_TITLE,
            (560.0, 340.0),
            parent,
            move |cx, window| {
                let mutation =
                    AsyncMutation::<DeviceKeyReplacementOutcome>::use_in(cx, runtime.clone());
                let prompts = crate::app::AuthenticationPromptCommandHandler::new(
                    windows_reactor::WinUIDispatcher::for_current_thread()
                        .expect("replacement rendering runs on the native UI thread")
                        .marshaller(),
                    runtime.clone(),
                    coordinator.clone(),
                );
                let handler = ReplaceDeviceKeyWindowCommandHandler {
                    profile_application: Arc::clone(&profile_application),
                    id: id.clone(),
                    window: window.clone(),
                    parent,
                    on_toast: on_toast.clone(),
                    mutation: mutation.clone(),
                    prompts,
                };
                Self::render(cx, handler, mutation, window)
            },
        )
    }

    fn render(
        cx: &mut RenderCx,
        handler: ReplaceDeviceKeyWindowCommandHandler,
        mutation: AsyncMutation<DeviceKeyReplacementOutcome>,
        window: AuxiliaryWindow,
    ) -> Element {
        let (state, set_state) = cx.use_reducer(ReplaceDeviceKeyState::default());

        let mutation_runner = handler.clone();
        let mutation_set_state = set_state.clone();
        mutation.on_change(move |mutation_state| {
            let result = match mutation_state {
                MutationState::Success(outcome) => outcome,
                MutationState::Error(error) => DeviceKeyReplacementOutcome::Failed(error),
                MutationState::Idle | MutationState::Loading => return,
            };
            Self::dispatch(
                &mutation_set_state,
                &mutation_runner,
                ReplaceDeviceKeyEvent::ReplacementFinished(result),
            );
            mutation_runner.show();
        });
        let primary_set_state = set_state.clone();
        let primary_runner = handler.clone();
        let close_set_state = set_state.clone();
        let close_runner = handler.clone();
        let enter_set_state = set_state;
        let enter_runner = handler;

        let body = scroll_viewer(
            vstack((
                text_block(state.heading())
                    .font_size(24.0)
                    .semibold()
                    .wrap(),
                text_block(state.message()).wrap(),
            ))
            .spacing(16.0)
            .padding(Thickness::uniform(24.0)),
        )
        .grid_row(0);
        let footer = border(
            hstack((
                button(state.primary_label())
                    .min_width(88.0)
                    .enabled(!state.is_busy())
                    .on_click(move || {
                        Self::dispatch(
                            &primary_set_state,
                            &primary_runner,
                            ReplaceDeviceKeyEvent::PrimaryRequested,
                        );
                    }),
                button(state.close_label())
                    .min_width(88.0)
                    .enabled(!state.is_busy())
                    .on_click(move || {
                        Self::dispatch(
                            &close_set_state,
                            &close_runner,
                            ReplaceDeviceKeyEvent::CloseRequested,
                        );
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
        let content = grid((body, footer))
            .rows([GridLength::STAR, GridLength::Auto])
            .keyboard_accelerator(KeyboardAccelerator::new(
                VirtualKey::Enter,
                VirtualKeyModifiers::None,
                move || {
                    Self::dispatch(
                        &enter_set_state,
                        &enter_runner,
                        ReplaceDeviceKeyEvent::PrimaryRequested,
                    );
                },
            ));
        window.content(content.into(), !state.is_busy())
    }

    fn dispatch(
        set_state: &Updater<ReplaceDeviceKeyState>,
        handler: &ReplaceDeviceKeyWindowCommandHandler,
        event: ReplaceDeviceKeyEvent,
    ) {
        let (_, command) = crate::app::ui::reduce(set_state, event);
        if let Some(command) = command {
            handler.handle(command);
        }
    }
}
