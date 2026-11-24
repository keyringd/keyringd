use keyringd_core::model::ProfileSshIdentity;
use keyringd_ui_core::text as ui_text;
use std::sync::Arc;

use crate::app::WindowsApplication;

use crate::app::ui::util::AsyncMutation;

use keyringd_core::model::ProfileId;

use windows_reactor::{
    AccessibilityExt, BackgroundExt, Element, GridChildExt, GridLength, HorizontalAlignment,
    InfoBar, InfoBarSeverity, InputExt, KeyboardAccelerator, LayoutExt, PaddingExt, RadioButton,
    TextStyleExt, ThemeRef, Thickness, Updater, VirtualKey, VirtualKeyModifiers, border, button,
    grid, hstack, scroll_viewer, text_block, text_box, vstack,
};

use crate::app::ui::reduce;

use crate::app::ui::AuxiliaryWindow;
use crate::app::ui::ssh_connection_test_window::SshConnectionTestCommandHandler;
use keyringd_ui_core::ssh_connection_test_window::SshConnectionTestEvent;
use keyringd_ui_core::ssh_connection_test_window::SshConnectionTestState;

pub(crate) struct SshConnectionTestWindow;

impl SshConnectionTestWindow {
    pub(crate) fn open(
        profile_application: Arc<WindowsApplication>,
        runtime: tokio::runtime::Handle,
        id: ProfileId,
        parent: usize,
        #[cfg(feature = "test-support")] test_open_ssh: Option<
            Arc<crate::app::ui_test_support::ControlledOpenSsh>,
        >,
    ) -> anyhow::Result<()> {
        let profile = profile_application.profile(&id)?;
        let identity_kinds =
            ProfileSshIdentity::available_kinds_at(&profile, std::time::SystemTime::now());
        let profile_name = profile.display_name().to_owned();
        let initial_state = SshConnectionTestState::new(identity_kinds.clone());
        AuxiliaryWindow::open_modal(
            keyringd_ui_core::ssh_connection_test_window::profile_heading(&profile_name),
            (640.0, 500.0),
            parent,
            move |cx, window| {
                let (state, set_state) = cx.use_reducer(initial_state.clone());
                let mutation = AsyncMutation::use_in(cx, runtime.clone());
                let task = cx.use_ref(None::<tokio::task::AbortHandle>);
                let cleanup_task = task.clone();
                cx.use_effect_with_cleanup((), move || {
                    Some(move || {
                        if let Some(task) = cleanup_task.get_cloned() {
                            task.abort();
                        }
                    })
                });
                let handler = SshConnectionTestCommandHandler::new(
                    Arc::clone(&profile_application),
                    profile.id().clone(),
                    window.clone(),
                    mutation.clone(),
                    task,
                    set_state.clone(),
                    #[cfg(feature = "test-support")]
                    test_open_ssh.clone(),
                );
                let result_set_state = set_state.clone();
                let command_handler = handler.clone();
                mutation.on_change(move |mutation_state| {
                    Self::dispatch(
                        &result_set_state,
                        &command_handler,
                        SshConnectionTestEvent::TestFinished(match mutation_state {
                            windows_reactor::MutationState::Success(result) => Ok(result),
                            windows_reactor::MutationState::Error(error) => Err(error),
                            _ => return,
                        }),
                    )
                });
                Self::render(&state, set_state, handler, &profile_name, window)
            },
        )
    }

    fn render(
        state: &SshConnectionTestState,
        set_state: Updater<SshConnectionTestState>,
        handler: SshConnectionTestCommandHandler,
        profile_name: &str,
        window: AuxiliaryWindow,
    ) -> Element {
        let choices: Vec<Element> = state
            .identities()
            .iter()
            .enumerate()
            .map(|(index, identity)| {
                let identity = *identity;
                let title = keyringd_ui_core::ssh_connection_test_window::identity_title(identity);
                let description =
                    keyringd_ui_core::ssh_connection_test_window::identity_description(
                        identity,
                        ui_text::SSH_CONNECTION_TEST_TPM_KEY_DESCRIPTION,
                    );
                let selected_set_state = set_state.clone();
                let selected_runner = handler.clone();
                let row_set_state = set_state.clone();
                let row_runner = handler.clone();
                let can_edit = state.can_edit();
                vstack((
                    RadioButton::new(title)
                        .automation_id(format!("ssh-test-identity-{index}"))
                        .group("connection-test-identity")
                        .checked(identity == state.identity())
                        .enabled(state.can_edit())
                        .on_checked(move || {
                            Self::dispatch(
                                &selected_set_state,
                                &selected_runner,
                                SshConnectionTestEvent::IdentitySelected(identity),
                            )
                        }),
                    text_block(description)
                        .foreground(ThemeRef::SecondaryText)
                        .wrap()
                        .margin(Thickness {
                            left: 28.0,
                            top: 0.0,
                            right: 0.0,
                            bottom: 0.0,
                        }),
                ))
                .on_tapped(move || {
                    if can_edit {
                        Self::dispatch(
                            &row_set_state,
                            &row_runner,
                            SshConnectionTestEvent::IdentitySelected(identity),
                        );
                    }
                })
                .into()
            })
            .collect();
        let error: Element = if state.error().is_none() {
            Element::Empty
        } else {
            InfoBar::new(state.error().unwrap_or_default())
                .severity(InfoBarSeverity::Error)
                .is_open(true)
                .into()
        };
        let test_set_state = set_state.clone();
        let test_runner = handler.clone();
        let start: std::rc::Rc<dyn Fn()> = std::rc::Rc::new(move || {
            Self::dispatch(
                &test_set_state,
                &test_runner,
                SshConnectionTestEvent::TestRequested,
            )
        });
        let enter = start.clone();
        let host_set_state = set_state.clone();
        let host_runner = handler.clone();
        let close_set_state = set_state.clone();
        let close_runner = handler;
        let busy = state.is_testing();
        let heading = vstack((
            text_block(keyringd_ui_core::ssh_connection_test_window::profile_heading(profile_name))
                .font_size(24.0)
                .semibold()
                .wrap(),
            text_block(ui_text::SSH_CONNECTION_TEST_DESCRIPTION)
                .foreground(ThemeRef::SecondaryText)
                .wrap(),
        ))
        .spacing(8.0);
        let form = vstack((
            text_box(state.host())
                .automation_id("ssh-test-host")
                .header(ui_text::SSH_CONNECTION_TEST_HOST_LABEL)
                .placeholder_text(ui_text::SSH_CONNECTION_TEST_HOST_PLACEHOLDER)
                .enabled(state.can_edit())
                .on_text_changed(move |host| {
                    Self::dispatch(
                        &host_set_state,
                        &host_runner,
                        SshConnectionTestEvent::HostChanged(host),
                    )
                })
                .keyboard_accelerator(KeyboardAccelerator::new(
                    VirtualKey::Enter,
                    VirtualKeyModifiers::None,
                    move || enter(),
                )),
            vstack((
                text_block(ui_text::SSH_CONNECTION_TEST_IDENTITY_LABEL).semibold(),
                vstack(choices)
                    .automation_id("ssh-test-identities")
                    .spacing(10.0),
            ))
            .spacing(8.0),
            error,
        ))
        .spacing(20.0);
        let body = grid((heading.grid_row(0), scroll_viewer(form).grid_row(1)))
            .rows([GridLength::Auto, GridLength::STAR])
            .row_spacing(24.0)
            .padding(Thickness::uniform(24.0))
            .grid_row(0);
        let footer = border(
            hstack((
                button(if busy {
                    ui_text::SSH_CONNECTION_TEST_TESTING_LABEL
                } else {
                    ui_text::SSH_CONNECTION_TEST_TEST
                })
                .automation_id("ssh-test-start")
                .min_width(100.0)
                .accent()
                .enabled(state.can_test())
                .on_click(move || start()),
                button(if busy {
                    ui_text::CANCEL
                } else {
                    ui_text::CLOSE
                })
                .min_width(88.0)
                .enabled(!busy)
                .on_click(move || {
                    Self::dispatch(
                        &close_set_state,
                        &close_runner,
                        SshConnectionTestEvent::CloseRequested,
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
        window.content(
            grid((body, footer))
                .rows([GridLength::STAR, GridLength::Auto])
                .into(),
            !state.is_testing(),
        )
    }

    fn dispatch(
        set_state: &Updater<SshConnectionTestState>,
        handler: &SshConnectionTestCommandHandler,
        event: SshConnectionTestEvent,
    ) {
        let (_, command) = reduce(set_state, event);
        if let Some(command) = command {
            handler.handle(command);
        }
    }
}
