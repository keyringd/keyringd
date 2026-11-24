use keyringd_ui_core::key_operation::KeyOperationCoordinator;
use keyringd_ui_core::runtime::UiEventTarget;
use keyringd_ui_core::text as ui_text;
use std::sync::Arc;

use crate::app::WindowsApplication;

use crate::app::ui::util::UiEventSink;
use keyringd_core::model::ProfileId;

use windows_reactor::{
    BackgroundExt, Button, Element, GridChildExt, GridLength, HorizontalAlignment, InfoBar,
    InfoBarSeverity, LayoutExt, PaddingExt, RenderCx, ScrollBarVisibility, TextStyleExt, ThemeRef,
    Thickness, Updater, VerticalAlignment, border, button, grid, scroll_viewer, text_block, vstack,
};

use crate::app::ui::reduce;

use crate::app::ui::AuxiliaryWindow;

use crate::app::ui::profile_details_window::ProfileDetailsWindowCommandHandler;
use keyringd_ui_core::profile_details_window::ProfileDetailsEvent;
use keyringd_ui_core::profile_details_window::ProfileDetailsState;

pub(crate) struct ProfileDetailsWindow;

impl ProfileDetailsWindow {
    pub(crate) fn open_root(
        profile_application: Arc<WindowsApplication>,
        coordinator: Arc<KeyOperationCoordinator>,
        runtime: tokio::runtime::Handle,
        id: ProfileId,
    ) -> anyhow::Result<()> {
        Self::open(profile_application, coordinator, runtime, id, None)
    }

    pub(crate) fn open_modal(
        profile_application: Arc<WindowsApplication>,
        coordinator: Arc<KeyOperationCoordinator>,
        runtime: tokio::runtime::Handle,
        id: ProfileId,
        parent: usize,
    ) -> anyhow::Result<()> {
        Self::open(profile_application, coordinator, runtime, id, Some(parent))
    }

    fn open(
        profile_application: Arc<WindowsApplication>,
        coordinator: Arc<KeyOperationCoordinator>,
        runtime: tokio::runtime::Handle,
        id: ProfileId,
        parent: Option<usize>,
    ) -> anyhow::Result<()> {
        let initial_state = ProfileDetailsState::new(&profile_application.profile(&id)?);
        let render = move |cx: &mut RenderCx, window: AuxiliaryWindow| {
            let (feedback, on_toast) = crate::app::ui::toast::use_toast(cx);
            let handler = ProfileDetailsWindowCommandHandler::new(
                Arc::clone(&profile_application),
                Arc::clone(&coordinator),
                runtime.clone(),
                id.clone(),
                window.clone(),
                on_toast,
            );
            grid((
                Self::render(cx, initial_state.clone(), handler, window),
                feedback,
            ))
            .into()
        };
        match parent {
            Some(parent) => AuxiliaryWindow::open_modal(
                ui_text::PROFILE_DETAILS_WINDOW_TITLE,
                (640.0, 580.0),
                parent,
                render,
            ),
            None => AuxiliaryWindow::open_root(
                ui_text::PROFILE_DETAILS_WINDOW_TITLE,
                (640.0, 580.0),
                render,
            ),
        }
    }

    fn render(
        cx: &mut RenderCx,
        initial_state: ProfileDetailsState,
        handler: ProfileDetailsWindowCommandHandler,
        window: AuxiliaryWindow,
    ) -> Element {
        let (state, set_state) = cx.use_reducer(initial_state);
        let updates = UiEventSink::use_in(cx);
        let update_state = set_state.clone();
        let update_runner = handler.clone();
        updates.bind(UiEventTarget::new(move |event| {
            Self::dispatch(&update_state, &update_runner, event)
        }));
        cx.use_effect_with_cleanup((), {
            let updates = updates.sender();
            let handler = handler.clone();
            move || {
                let task = handler.subscribe_profile_changes(updates);
                Some(move || task.abort())
            }
        });
        let status = state.status();
        let protection = state.device_key_local_authentication();
        let trust_set_state = set_state.clone();
        let trust_runner = handler.clone();
        let replace_set_state = set_state.clone();
        let replace_runner = handler.clone();
        let close_set_state = set_state;
        let close_runner = handler;

        let error = state
            .error()
            .map(|error| {
                InfoBar::new(error)
                    .severity(InfoBarSeverity::Error)
                    .is_open(true)
                    .into()
            })
            .unwrap_or(Element::Empty);
        let summary = Self::card(
            vstack((
                Self::detail(ui_text::PROFILE_DETAILS_STATUS, status),
                Self::detail(
                    ui_text::PROFILE_DETAILS_SIGNATURE_FORMAT,
                    state.device_key_signature_algorithm(),
                ),
                Self::detail(ui_text::PROFILE_DETAILS_LOCAL_AUTHENTICATION, protection),
            ))
            .spacing(14.0)
            .into(),
        );
        let access = vstack((
            text_block(ui_text::PROFILE_DETAILS_ACCESS).semibold(),
            button(ui_text::TRUST_ENTRIES_MENU_ACTION).on_click(move || {
                Self::dispatch(
                    &trust_set_state,
                    &trust_runner,
                    ProfileDetailsEvent::TrustEntriesRequested,
                );
            }),
        ))
        .spacing(8.0);
        let replace = Self::action_row(
            ui_text::REPLACE_DEVICE_KEY_WINDOW_TITLE,
            &keyringd_ui_core::profile_details_window::replacement_description("TPM-backed"),
            button(ui_text::PROFILE_DETAILS_REPLACE).on_click(move || {
                Self::dispatch(
                    &replace_set_state,
                    &replace_runner,
                    ProfileDetailsEvent::ReplaceDeviceKeyRequested,
                );
            }),
        );
        let body = grid((
            text_block(state.heading())
                .font_size(24.0)
                .semibold()
                .wrap()
                .grid_row(0),
            scroll_viewer(
                vstack((
                    summary,
                    error,
                    access,
                    vstack((
                        text_block(ui_text::PROFILE_DETAILS_ADVANCED).semibold(),
                        replace,
                    ))
                    .spacing(8.0),
                ))
                .spacing(18.0),
            )
            .vertical_scroll_bar_visibility(ScrollBarVisibility::Auto)
            .grid_row(1),
        ))
        .rows([GridLength::Auto, GridLength::STAR])
        .row_spacing(20.0)
        .padding(Thickness::uniform(24.0))
        .grid_row(0);
        let footer = border(
            button(ui_text::CLOSE)
                .min_width(88.0)
                .horizontal_alignment(HorizontalAlignment::Right)
                .on_click(move || {
                    Self::dispatch(
                        &close_set_state,
                        &close_runner,
                        ProfileDetailsEvent::CloseRequested,
                    );
                }),
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
        let content = grid((body, footer)).rows([GridLength::STAR, GridLength::Auto]);
        window.content(content.into(), true)
    }

    fn dispatch(
        set_state: &Updater<ProfileDetailsState>,
        handler: &ProfileDetailsWindowCommandHandler,
        event: ProfileDetailsEvent,
    ) {
        let (_, command) = reduce(set_state, event);
        if let Some(command) = command
            && let Some(event) = handler.handle(command)
        {
            Self::dispatch(set_state, handler, event);
        }
    }

    fn detail(label: &str, value: &str) -> Element {
        vstack((
            text_block(label).semibold(),
            text_block(value).foreground(ThemeRef::SecondaryText).wrap(),
        ))
        .spacing(4.0)
        .into()
    }

    fn card(content: Element) -> Element {
        border(content)
            .padding(Thickness::uniform(16.0))
            .corner_radius(8.0)
            .background(ThemeRef::CardBackground)
            .border_brush(ThemeRef::CardStroke)
            .border_thickness(Thickness::uniform(1.0))
            .into()
    }

    fn action_row(title: &str, description: &str, action: Button) -> Element {
        Self::card(
            grid((
                vstack((
                    text_block(title).semibold(),
                    text_block(description)
                        .foreground(ThemeRef::SecondaryText)
                        .wrap(),
                ))
                .spacing(4.0)
                .grid_column(0),
                action
                    .min_width(88.0)
                    .vertical_alignment(VerticalAlignment::Center)
                    .grid_column(1),
            ))
            .columns([GridLength::STAR, GridLength::Auto])
            .column_spacing(20.0)
            .into(),
        )
    }
}
