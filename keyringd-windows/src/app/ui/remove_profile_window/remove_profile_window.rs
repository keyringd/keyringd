use keyringd_ui_core::text as ui_text;
use keyringd_ui_core::toast::ToastMessage;
use std::sync::Arc;

use keyringd_ui_core::remove_profile_window::{RemoveProfileEvent, RemoveProfileState};

use crate::app::WindowsApplication;

use crate::app::ui::util::AsyncMutation;

use keyringd_core::model::ProfileId;

use windows_reactor::{
    BackgroundExt, Element, GridChildExt, GridLength, HorizontalAlignment, LayoutExt,
    MutationState, PaddingExt, RenderCx, ThemeRef, Thickness, Updater, border, button, grid,
    hstack, scroll_viewer, text_block, vstack,
};

use crate::app::ui::AuxiliaryWindow;

use crate::app::ui::remove_profile_window::RemoveProfileWindowCommandHandler;

pub(crate) struct RemoveProfileWindow;

impl RemoveProfileWindow {
    pub(crate) fn open(
        profile_application: Arc<WindowsApplication>,
        runtime: tokio::runtime::Handle,
        id: ProfileId,
        parent: usize,
        notices: tokio::sync::watch::Sender<Option<ToastMessage>>,
    ) -> anyhow::Result<()> {
        let name = profile_application.profile(&id)?.display_name().to_owned();
        AuxiliaryWindow::open_modal(
            ui_text::REMOVE_PROFILE_WINDOW_TITLE,
            (520.0, 220.0),
            parent,
            move |cx, window| {
                let mutation = AsyncMutation::<()>::use_in(cx, runtime.clone());
                let handler = RemoveProfileWindowCommandHandler::new(
                    Arc::clone(&profile_application),
                    id.clone(),
                    window.clone(),
                    mutation.clone(),
                    name.clone(),
                    notices.clone(),
                );
                Self::render(cx, &name, handler, mutation, window)
            },
        )
    }

    fn render(
        cx: &mut RenderCx,
        name: &str,
        handler: RemoveProfileWindowCommandHandler,
        mutation: AsyncMutation<()>,
        window: AuxiliaryWindow,
    ) -> Element {
        let (state, set_state) = cx.use_reducer(RemoveProfileState::default());

        let mutation_runner = handler.clone();
        let mutation_set_state = set_state.clone();
        mutation.on_change(move |mutation_state| {
            let result = match mutation_state {
                MutationState::Success(()) => Ok(()),
                MutationState::Error(error) => Err(error),
                MutationState::Idle | MutationState::Loading => return,
            };
            Self::dispatch(
                &mutation_set_state,
                &mutation_runner,
                RemoveProfileEvent::RemovalFinished(result),
            );
        });
        let remove_set_state = set_state.clone();
        let remove_runner = handler.clone();
        let close_runner = handler;
        let close_set_state = set_state.clone();

        let body = scroll_viewer(
            vstack((
                text_block(state.heading(name))
                    .font_size(24.0)
                    .semibold()
                    .wrap(),
                text_block(state.message()).wrap(),
            ))
            .spacing(12.0)
            .padding(Thickness::uniform(24.0)),
        )
        .grid_row(0);
        let actions = border(
            hstack((
                button(state.remove_label())
                    .accent()
                    .min_width(88.0)
                    .enabled(state.can_remove())
                    .on_click(move || {
                        Self::dispatch(
                            &remove_set_state,
                            &remove_runner,
                            RemoveProfileEvent::RemovalRequested,
                        );
                    }),
                button(state.close_label())
                    .min_width(88.0)
                    .on_click(move || {
                        Self::dispatch(
                            &close_set_state,
                            &close_runner,
                            RemoveProfileEvent::CloseRequested,
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
        let content = grid((body, actions)).rows([GridLength::STAR, GridLength::Auto]);
        window.content(content.into(), true)
    }

    fn dispatch(
        set_state: &Updater<RemoveProfileState>,
        handler: &RemoveProfileWindowCommandHandler,
        event: RemoveProfileEvent,
    ) {
        let (_, command) = crate::app::ui::reduce(set_state, event);
        if let Some(command) = command {
            handler.handle(command);
        }
    }
}
