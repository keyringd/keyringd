use keyringd_ui_core::ssh_connection_test_window::{
    SshConnectionTestEvent, SshConnectionTestPresentation,
};
use keyringd_ui_core::text as ui_text;
use windows_reactor::{
    AccessibilityExt, BackgroundExt, Element, GridChildExt, GridLength, HorizontalAlignment,
    LayoutExt, PaddingExt, ScrollBarVisibility, TextStyleExt, ThemeRef, Thickness, border, button,
    grid, scroll_viewer, text_block, vstack,
};

use crate::app::ui::AuxiliaryWindow;
use crate::app::ui::toast::use_toast;
use keyringd_core::service::{SshConnectionTestReport, SshConnectionTestResult};
use keyringd_ui_core::runtime::UiEventTarget;
use keyringd_ui_core::toast::ToastMessage;
use std::rc::Rc;

pub(crate) struct ConnectionTestResultWindow;

impl ConnectionTestResultWindow {
    pub(crate) fn open(
        target: String,
        report: SshConnectionTestReport,
        parent: usize,
        on_event: Rc<dyn Fn(SshConnectionTestEvent) -> Option<ToastMessage>>,
    ) -> anyhow::Result<()> {
        let height = if matches!(report.result, SshConnectionTestResult::IdentityRejected) {
            600.0
        } else {
            320.0
        };
        AuxiliaryWindow::open_modal(
            ui_text::SSH_CONNECTION_TEST_RESULT_WINDOW_TITLE,
            (640.0, height),
            parent,
            move |cx, window| {
                let (feedback, on_toast) = use_toast(cx);
                let closed = on_event.clone();
                cx.use_effect_with_cleanup((), move || {
                    Some(move || {
                        closed(SshConnectionTestEvent::ResultClosed);
                    })
                });
                let close = window.clone();
                let body = scroll_viewer(
                    vstack((
                        text_block(keyringd_ui_core::ssh_connection_test_window::host_label(
                            &target,
                        ))
                        .automation_id("ssh-test-result-host")
                        .foreground(ThemeRef::SecondaryText)
                        .wrap(),
                        Self::result(&report, on_event.clone(), on_toast),
                    ))
                    .spacing(12.0)
                    .padding(Thickness::uniform(24.0)),
                )
                .grid_row(0);
                let footer = border(
                    button(ui_text::CLOSE)
                        .accent()
                        .min_width(88.0)
                        .horizontal_alignment(HorizontalAlignment::Right)
                        .on_click(move || close.close()),
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
                window.content(grid((content, feedback)).into(), true)
            },
        )
    }

    fn result(
        report: &SshConnectionTestReport,
        on_event: Rc<dyn Fn(SshConnectionTestEvent) -> Option<ToastMessage>>,
        on_toast: UiEventTarget<ToastMessage>,
    ) -> Element {
        let presentation = SshConnectionTestPresentation::new(report);
        let mut content: Vec<Element> = vec![
            text_block(presentation.heading)
                .automation_id("ssh-test-result-heading")
                .font_size(24.0)
                .semibold()
                .wrap()
                .into(),
            text_block(presentation.body)
                .automation_id("ssh-test-result-body")
                .wrap()
                .into(),
        ];
        if let Some((trust_entry, server_configuration)) = presentation.troubleshooting {
            content.push(
                text_block(format!(
                    "{}\n{}",
                    ui_text::SSH_CONNECTION_TEST_AUTHORIZED_KEYS_HEADING,
                    ui_text::SSH_CONNECTION_TEST_AUTHORIZED_KEYS_EXPLANATION
                ))
                .automation_id("ssh-test-authorized-keys")
                .wrap()
                .into(),
            );
            content.push(
                border(
                    scroll_viewer(
                        text_block(trust_entry.clone())
                            .automation_id("ssh-test-rejection-entry")
                            .selectable()
                            .font_family("Consolas"),
                    )
                    .horizontal_scroll_bar_visibility(ScrollBarVisibility::Hidden)
                    .vertical_scroll_bar_visibility(ScrollBarVisibility::Disabled),
                )
                .height(36.0)
                .padding(Thickness::xy(10.0, 7.0))
                .background(ThemeRef::ControlFill)
                .border_brush(ThemeRef::ControlStroke)
                .border_thickness(Thickness::uniform(1.0))
                .corner_radius(4.0)
                .into(),
            );
            content.push(
                button(ui_text::SSH_CONNECTION_TEST_COPY_ENTRY)
                    .automation_id("ssh-test-copy-entry")
                    .on_click(move || {
                        if let Some(message) = on_event(SshConnectionTestEvent::ResultCopyRequested)
                        {
                            on_toast.send(message);
                        }
                    })
                    .into(),
            );
            content.push(
                text_block(format!(
                    "{}\n{}",
                    ui_text::SSH_CONNECTION_TEST_SERVER_HEADING,
                    server_configuration
                ))
                .automation_id("ssh-test-server-guidance")
                .wrap()
                .into(),
            );
        }
        vstack(content).spacing(10.0).into()
    }
}
