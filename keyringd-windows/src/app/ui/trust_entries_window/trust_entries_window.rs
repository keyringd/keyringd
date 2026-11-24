use keyringd_ui_core::runtime::UiEventTarget;
use keyringd_ui_core::text as ui_text;
use keyringd_ui_core::trust_entries_window::TrustEntryKind;
use std::sync::Arc;

use crate::app::WindowsApplication;

use keyringd_core::model::ProfileId;

use windows_reactor::{
    BackgroundExt, Element, GridChildExt, GridLength, HorizontalAlignment, InfoBar, InputExt,
    KeyboardAccelerator, LayoutExt, PaddingExt, RenderCx, TextStyleExt, ThemeRef, Thickness,
    VirtualKey, VirtualKeyModifiers, border, button, grid, hstack, text_block, vstack,
};

use crate::app::ui::AuxiliaryWindow;

use crate::app::ui::trust_entries_window::TrustEntriesWindowCommandHandler;
use crate::app::ui::trust_entries_window::TrustEntryView;
use keyringd_ui_core::trust_entries_window::TrustEntriesEvent;

use crate::app::ui::toast::use_toast;

use crate::app::ui::reduce;
use crate::app::ui::util::UiEventSink;
use keyringd_ui_core::trust_entries_window::TrustEntriesState;
use windows_reactor::Updater;

pub(crate) struct TrustEntriesWindow;

impl TrustEntriesWindow {
    pub(crate) fn open_root(
        profile_application: Arc<WindowsApplication>,
        runtime: tokio::runtime::Handle,
        profile_id: ProfileId,
    ) -> anyhow::Result<()> {
        Self::open(profile_application, runtime, profile_id, None, None)
    }

    pub(crate) fn open_modal(
        profile_application: Arc<WindowsApplication>,
        runtime: tokio::runtime::Handle,
        profile_id: ProfileId,
        parent: usize,
    ) -> anyhow::Result<()> {
        Self::open(profile_application, runtime, profile_id, Some(parent), None)
    }

    pub(crate) fn open_after_protection_change(
        profile_application: Arc<WindowsApplication>,
        runtime: tokio::runtime::Handle,
        notice: keyringd_core::model::DeviceKeyRegistrationNotice,
        parent: usize,
    ) -> anyhow::Result<()> {
        Self::open(
            profile_application,
            runtime,
            notice.profile_id,
            Some(parent),
            Some(notice.message),
        )
    }

    fn open(
        profile_application: Arc<WindowsApplication>,
        runtime: tokio::runtime::Handle,
        profile_id: ProfileId,
        parent: Option<usize>,
        registration_notice: Option<String>,
    ) -> anyhow::Result<()> {
        let profile = profile_application.profile(&profile_id)?;
        let initial_state =
            TrustEntriesState::from_profile(&profile).with_notice(registration_notice);
        let title = initial_state.heading();
        let render = move |cx: &mut RenderCx, window| {
            let (feedback, on_toast) = use_toast(cx);
            let handler = TrustEntriesWindowCommandHandler::new(
                Arc::clone(&profile_application),
                runtime.clone(),
                profile_id.clone(),
                window,
                on_toast,
            );
            Self::render(cx, initial_state.clone(), handler, feedback)
        };
        match parent {
            Some(parent) => AuxiliaryWindow::open_modal(title, (760.0, 680.0), parent, render),
            None => AuxiliaryWindow::open_root(title, (760.0, 680.0), render),
        }
    }

    fn render(
        cx: &mut RenderCx,
        initial_state: TrustEntriesState,
        handler: TrustEntriesWindowCommandHandler,
        feedback: Element,
    ) -> Element {
        let (state, set_state) = cx.use_reducer(initial_state);
        let event_runner = handler.clone();
        let event_state = set_state.clone();
        let on_event: UiEventTarget<TrustEntriesEvent> =
            UiEventTarget::new(move |event| Self::dispatch(&event_state, &event_runner, event));
        let updates = UiEventSink::use_in(cx);
        updates.bind(on_event.clone());
        cx.use_effect_with_cleanup((), {
            let updates = updates.sender();
            let handler = handler.clone();
            move || {
                let task = handler.subscribe_profile_changes(updates);
                Some(move || task.abort())
            }
        });
        let ca = TrustEntryView::render(
            TrustEntryViewProps {
                kind: TrustEntryKind::CertificateAuthority,
                description: ui_text::TRUST_ENTRIES_CA_DESCRIPTION,
                value: state.value(TrustEntryKind::CertificateAuthority),
            },
            on_event.clone(),
        );
        let device = TrustEntryView::render(
            TrustEntryViewProps {
                kind: TrustEntryKind::DeviceKey,
                description: ui_text::TRUST_ENTRIES_TPM_KEY_DESCRIPTION,
                value: state.value(TrustEntryKind::DeviceKey),
            },
            on_event.clone(),
        );
        let passkey = TrustEntryView::render(
            TrustEntryViewProps {
                kind: TrustEntryKind::Passkey,
                description: ui_text::TRUST_ENTRIES_PASSKEY_DESCRIPTION,
                value: state.value(TrustEntryKind::Passkey),
            },
            on_event.clone(),
        );
        let copy_event = on_event.clone();
        let copy_both = button(TrustEntryKind::DirectKeys.copy_label()).on_click(move || {
            copy_event.send(TrustEntriesEvent::CopyRequested(TrustEntryKind::DirectKeys))
        });
        let close_event = on_event.clone();
        let escape_event = on_event.clone();
        let body = vstack((
            text_block(state.heading()).font_size(28.0).bold(),
            state
                .registration_notice()
                .map(|message| {
                    InfoBar::new(ui_text::TRUST_ENTRIES_REGISTRATION_UPDATE)
                        .message(message.to_owned())
                        .warning()
                        .is_closable(false)
                        .into()
                })
                .unwrap_or(Element::Empty),
            text_block(ui_text::TRUST_ENTRIES_DESCRIPTION)
                .wrap()
                .foreground(ThemeRef::SecondaryText),
            vstack((
                text_block(ui_text::TRUST_ENTRIES_CERTIFICATE_SERVERS)
                    .font_size(20.0)
                    .semibold(),
                text_block(ui_text::TRUST_ENTRIES_CERTIFICATE_DESCRIPTION)
                    .wrap()
                    .foreground(ThemeRef::SecondaryText),
                ca,
            ))
            .spacing(8.0),
            vstack((
                text_block(ui_text::TRUST_ENTRIES_DIRECT_SERVERS)
                    .font_size(20.0)
                    .semibold(),
                text_block(ui_text::TRUST_ENTRIES_DIRECT_DESCRIPTION)
                    .wrap()
                    .foreground(ThemeRef::SecondaryText),
                device,
                passkey,
                hstack((copy_both,)).horizontal_alignment(HorizontalAlignment::Right),
            ))
            .spacing(8.0),
        ))
        .spacing(16.0)
        .padding(Thickness::xy(32.0, 24.0))
        .horizontal_alignment(HorizontalAlignment::Stretch)
        .grid_row(0);
        let footer = border(
            button(ui_text::CLOSE)
                .min_width(88.0)
                .on_click(move || close_event.send(TrustEntriesEvent::CloseRequested))
                .horizontal_alignment(HorizontalAlignment::Right),
        )
        .padding(Thickness::xy(32.0, 16.0))
        .background(ThemeRef::LayerFill)
        .border_brush(ThemeRef::CardStroke)
        .border_thickness(Thickness {
            left: 0.0,
            top: 1.0,
            right: 0.0,
            bottom: 0.0,
        })
        .grid_row(1);
        let content = grid((windows_reactor::scroll_viewer(body).grid_row(0), footer))
            .rows([GridLength::STAR, GridLength::Auto]);
        grid((content, feedback))
            .keyboard_accelerator(KeyboardAccelerator::new(
                VirtualKey::Escape,
                VirtualKeyModifiers::None,
                move || escape_event.send(TrustEntriesEvent::CloseRequested),
            ))
            .into()
    }

    fn dispatch(
        set_state: &Updater<TrustEntriesState>,
        handler: &TrustEntriesWindowCommandHandler,
        event: TrustEntriesEvent,
    ) {
        let (_, command) = reduce(set_state, event);
        if let Some(command) = command {
            handler.handle(command);
        }
    }
}

use keyringd_ui_core::trust_entries_window::TrustEntryViewProps;
