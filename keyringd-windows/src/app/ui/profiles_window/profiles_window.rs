use keyringd_ui_core::device_key_protection_window::DeviceKeyProtectionResult;
use keyringd_ui_core::key_operation::KeyOperationCoordinator;
use keyringd_ui_core::runtime::UiEventReceiver;
use keyringd_ui_core::text as ui_text;
use keyringd_ui_core::toast::ToastMessage;
use std::sync::Arc;

use crate::app::WindowsApplication;

use crate::app::ui::util::AsyncMutation;
use crate::app::ui::util::{CreatedReactorWindowLocator, center_on_cursor_monitor};

use keyringd_core::model::ProfileId;

use windows::Win32::Foundation::HWND;

use windows::Win32::UI::WindowsAndMessaging::{IsWindow, SetForegroundWindow};

use windows_reactor::{
    AccessibilityExt, Backdrop, DispatcherTimer, Element, GridChildExt, GridLength,
    HorizontalAlignment, Icon, InfoBar, InfoBarSeverity, LayoutExt, MutationState, PaddingExt,
    ReactorWindow, RenderCx, TextBoxHandle, TextStyleExt, Thickness, border, button, grid,
    text_block, vstack,
};

use crate::app::ui::profile_details_window::ProfileDetailsWindow;
use crate::app::ui::profile_list::ProfileListView;
use crate::app::ui::ssh_client_configuration_window::SshClientConfigurationWindow;
use crate::app::ui::trust_entries_window::TrustEntriesWindow;

use crate::app::ui::profiles_window::ProfilesWindowCommandHandler;
use keyringd_ui_core::profiles_window::ProfilesWindowEvent;
use keyringd_ui_core::profiles_window::ProfilesWindowState;

// A 14-DIP gear for the caption-sized footer action.
const SSH_CLIENT_SETTINGS_ICON: &str = "F0 M12.2,5.21 13.98,6.02 13.98,7.98 12.2,8.79 \
    11.94,9.41 12.63,11.24 11.24,12.63 9.41,11.94 8.79,12.2 7.98,13.98 6.02,13.98 \
    5.21,12.2 4.59,11.94 2.76,12.63 1.37,11.24 2.06,9.41 1.8,8.79 .02,7.98 .02,6.02 \
    1.8,5.21 2.06,4.59 1.37,2.76 2.76,1.37 4.59,2.06 5.21,1.8 6.02,.02 7.98,.02 \
    8.79,1.8 9.41,2.06 11.24,1.37 12.63,2.76 11.94,4.59 Z \
    M7,4.65 A2.35,2.35 0 1 1 7,9.35 A2.35,2.35 0 1 1 7,4.65 Z";

pub(crate) struct ProfilesWindow {
    profile_application: Arc<WindowsApplication>,
    coordinator: Arc<KeyOperationCoordinator>,
    runtime: tokio::runtime::Handle,
    configure_ssh: SshClientConfigurationWindow,
    window_handle: std::sync::atomic::AtomicUsize,
    events: std::sync::Mutex<Option<keyringd_ui_core::runtime::UiEventSender<ProfilesWindowEvent>>>,
    pub(super) removal_notices: tokio::sync::watch::Sender<Option<ToastMessage>>,
    #[cfg(feature = "test-support")]
    pub(super) test_open_ssh: Option<Arc<crate::app::ui_test_support::ControlledOpenSsh>>,
}

impl ProfilesWindow {
    pub(crate) fn new(
        profile_application: Arc<WindowsApplication>,
        coordinator: Arc<KeyOperationCoordinator>,
        runtime: tokio::runtime::Handle,
        configure_ssh: SshClientConfigurationWindow,
        #[cfg(feature = "test-support")] test_open_ssh: Option<
            Arc<crate::app::ui_test_support::ControlledOpenSsh>,
        >,
    ) -> Self {
        Self {
            profile_application,
            coordinator,
            runtime,
            configure_ssh,
            window_handle: std::sync::atomic::AtomicUsize::new(0),
            events: std::sync::Mutex::new(None),
            removal_notices: tokio::sync::watch::channel(None).0,
            #[cfg(feature = "test-support")]
            test_open_ssh,
        }
    }

    pub(crate) fn show(self: &Arc<Self>) -> anyhow::Result<()> {
        let existing = self.window_handle();
        if existing != 0
            && unsafe { IsWindow(Some(HWND(existing as *mut core::ffi::c_void))) }.as_bool()
        {
            unsafe {
                let _ = SetForegroundWindow(HWND(existing as *mut core::ffi::c_void));
            }
            return Ok(());
        }
        let opened_window = Arc::clone(self);
        let rendered_window = Arc::clone(self);
        let key_icon = tempfile::Builder::new().suffix(".png").tempfile()?;
        std::fs::write(
            key_icon.path(),
            keyringd_ui_core::profile_list::PROFILES_EMPTY_KEY_ICON,
        )?;
        let key_icon_uri = format!("file:///{}", key_icon.path().display()).replace('\\', "/");
        let created_window = CreatedReactorWindowLocator::begin()?;
        ReactorWindow::new()
            .title("Keyringd")
            .inner_size(700.0, 400.0)
            .backdrop(Backdrop::Mica)
            .render(move |cx| {
                // WinUI loads the embedded artwork from this file while the window is open.
                let _ = &key_icon;
                Arc::clone(&rendered_window).render(cx, &key_icon_uri)
            })?;
        let handle = created_window.finish()?;
        center_on_cursor_monitor(handle);
        crate::app::ui::util::apply_application_icon(handle)?;
        opened_window
            .window_handle
            .store(handle.0 as usize, std::sync::atomic::Ordering::Release);
        Ok(())
    }

    fn render(self: Arc<Self>, cx: &mut RenderCx, key_icon_uri: &str) -> Element {
        let (model, set_model) = cx.use_reducer(ProfilesWindowState::new(
            self.profile_application.profiles(),
        ));
        let profiles = model.profiles();
        let profile_events = crate::app::ui::util::UiEventSink::use_in(cx);
        let mut notices = self.removal_notices.subscribe();
        let initial_notice = notices.borrow_and_update().clone();
        let (notice, set_notice) = cx.use_async_state(initial_notice);
        let notice_to_expire = notice.clone();
        let notice_sender = self.removal_notices.clone();
        cx.use_effect_with_cleanup(notice.clone(), move || {
            let timer = match notice_to_expire {
                Some(ToastMessage::Success(_)) => {
                    DispatcherTimer::new_one_shot(std::time::Duration::from_secs(3), move || {
                        notice_sender.send_replace(None);
                    })
                    .map_err(|error| tracing::warn!(%error, "could not schedule notice dismissal"))
                    .ok()
                }
                _ => None,
            };
            Some(move || drop(timer))
        });
        let runtime = self.runtime.clone();
        cx.use_effect_with_cleanup((), move || {
            let (initial, mut notices) = UiEventReceiver::from_tokio_watch(&runtime, notices);
            set_notice.call(initial);
            let task = runtime.spawn(async move {
                while let Some(value) = notices.receive().await {
                    set_notice.call(value);
                }
            });
            Some(move || task.abort())
        });
        let edit_name_ref = cx.use_element_ref::<TextBoxHandle>();
        let (protection_feedback, on_toast) = crate::app::ui::toast::use_toast(cx);
        let configure_ssh = {
            let events = crate::app::ui::util::UiEventSink::use_in(cx);
            let window = Arc::clone(&self);
            let on_toast = on_toast.clone();
            events.bind(keyringd_ui_core::runtime::UiEventTarget::new(move |()| {
                if let Err(error) = window.configure_ssh.open(window.window_handle()) {
                    on_toast.send(ToastMessage::Error(keyringd_core::util::report_user_error(
                        &error,
                        ui_text::OPEN_WINDOW_FAILED,
                    )));
                }
            }));
            // Let the button invocation return before entering the native modal loop.
            let open = events.sender();
            button(ui_text::APP_MENU_SSH_CLIENT_CONFIGURATION)
                .icon(Icon::path(SSH_CLIENT_SETTINGS_ICON))
                .font_size(12.0)
                .min_height(32.0)
                .subtle()
                .automation_id("profiles.configure-ssh-clients")
                .horizontal_alignment(HorizontalAlignment::Left)
                .on_click(move || open.send(()))
                .grid_row(2)
        };
        let profile_operation =
            AsyncMutation::<Option<DeviceKeyProtectionResult>>::use_in(cx, self.runtime.clone());
        let profile_operation_owner = self.window_handle();

        cx.use_effect_with_cleanup((), {
            let profile_changes = self.profile_application.subscribe_profile_changes();
            let runtime = self.runtime.clone();
            let profile_events = profile_events.sender();
            move || {
                let (initial, mut profile_changes) =
                    UiEventReceiver::from_tokio_watch(&runtime, profile_changes);
                profile_events.send(ProfilesWindowEvent::ProfilesChanged(initial));
                let task = runtime.spawn(async move {
                    while let Some(profiles) = profile_changes.receive().await {
                        profile_events.send(ProfilesWindowEvent::ProfilesChanged(profiles));
                    }
                });
                Some(move || task.abort())
            }
        });

        profile_operation.on_change({
            let on_toast = on_toast.clone();
            let set_model = set_model.clone();
            let profile_application = self.profile_application.clone();
            let runtime = self.runtime.clone();
            move |profile_operation_state| match profile_operation_state {
                MutationState::Success(result) => {
                    if let Some(result) = &result {
                        on_toast.send(ToastMessage::Success(result.notice.clone()));
                    }
                    ProfilesWindowCommandHandler::complete_profile_operation(
                        &set_model,
                        profile_operation_owner,
                        Ok(()),
                    );
                    if let Some(notice) = result.and_then(|result| result.registration_notice) {
                        let owner = HWND(profile_operation_owner as *mut core::ffi::c_void);
                        if unsafe { IsWindow(Some(owner)) }.as_bool() {
                            let result = TrustEntriesWindow::open_after_protection_change(
                                profile_application.clone(), runtime.clone(), notice, profile_operation_owner,
                            ).map_err(|error| keyringd_core::util::report_user_error(&error, "Local authentication changed, but Public Keys could not be opened. Open Public Keys from the Profile menu to update your server registration."));
                            ProfilesWindowCommandHandler::apply_result(&set_model, result);
                        }
                    }
                }
                MutationState::Error(error) => {
                    ProfilesWindowCommandHandler::complete_profile_operation(
                        &set_model,
                        profile_operation_owner,
                        Err(error),
                    )
                }
                MutationState::Idle | MutationState::Loading => {}
            }
        });

        let on_window_event = ProfilesWindowCommandHandler::new(
            Arc::clone(&self),
            profile_operation.clone(),
            on_toast,
            profile_events.sender(),
        )
        .event_handler(set_model.clone());
        profile_events.bind(on_window_event.clone());
        *self.events.lock().unwrap() = Some(profile_events.sender());
        cx.use_effect((), {
            let events = profile_events.sender();
            move || events.send(ProfilesWindowEvent::Opened)
        });
        let on_list_event = on_window_event.map(ProfilesWindowEvent::from);
        let content = ProfileListView::render(
            keyringd_ui_core::profile_list::ProfileListProps {
                saving: model.is_saving(),
                creation_enabled: model.can_add_profile(),
                profiles: profiles.to_vec(),
                revision: model.profiles_revision(),
                editing: model
                    .editing()
                    .map(|(id, name)| (id.clone(), name.to_owned())),
            },
            edit_name_ref.clone(),
            on_list_event,
            key_icon_uri,
        );
        let hardware_bar: Element = if let Some(message) = model.device_key_error() {
            InfoBar::new(message)
                .severity(InfoBarSeverity::Warning)
                .is_open(true)
                .is_closable(false)
                .into()
        } else {
            Element::Empty
        };
        let error_bar: Element = if let Some(error_message) = model.error() {
            InfoBar::new(error_message)
                .severity(InfoBarSeverity::Error)
                .is_open(true)
                .into()
        } else {
            border(text_block("")).height(0.0).into()
        };
        let notice_bar: Element = if let Some(notice) = notice {
            let (message, severity) = match notice {
                ToastMessage::Success(message) => (message, InfoBarSeverity::Success),
                ToastMessage::Error(message) => (message, InfoBarSeverity::Error),
            };
            let notices = self.removal_notices.clone();
            InfoBar::new(message)
                .severity(severity)
                .is_open(true)
                .on_closed(move || {
                    notices.send_replace(None);
                })
                .into()
        } else {
            Element::Empty
        };
        let edit_id = model.editing().map(|(id, _)| id.clone());
        cx.use_effect(edit_id.clone(), move || {
            if edit_id.is_some() {
                let _ = edit_name_ref.focus();
            }
        });
        let content: Element = grid((
            vstack((hardware_bar, error_bar, notice_bar))
                .spacing(8.0)
                .grid_row(0),
            border(content).grid_row(1),
            configure_ssh,
        ))
        .rows([GridLength::Auto, GridLength::STAR, GridLength::Auto])
        .row_spacing(if profiles.is_empty() { 0.0 } else { 12.0 })
        .padding(Thickness::xy(32.0, 24.0))
        .horizontal_alignment(HorizontalAlignment::Stretch)
        .into();
        grid((content, protection_feedback)).into()
    }

    pub(super) fn window_handle(&self) -> usize {
        self.window_handle
            .load(std::sync::atomic::Ordering::Acquire)
    }

    pub(crate) fn application(&self) -> Arc<WindowsApplication> {
        Arc::clone(&self.profile_application)
    }

    pub(super) fn coordinator(&self) -> Arc<KeyOperationCoordinator> {
        Arc::clone(&self.coordinator)
    }

    pub(super) fn runtime(&self) -> tokio::runtime::Handle {
        self.runtime.clone()
    }

    pub(crate) fn add_profile(self: &Arc<Self>) -> anyhow::Result<()> {
        self.show()?;
        if let Some(events) = self.events.lock().unwrap().as_ref() {
            events.send(ProfilesWindowEvent::AddRequested);
        }
        Ok(())
    }

    pub(crate) fn test_connection(self: &Arc<Self>, id: ProfileId) -> anyhow::Result<()> {
        self.show()?;
        crate::app::ui::ssh_connection_test_window::SshConnectionTestWindow::open(
            self.application(),
            self.runtime(),
            id,
            self.window_handle(),
            #[cfg(feature = "test-support")]
            self.test_open_ssh.clone(),
        )
    }

    pub(crate) fn show_profile_details(self: &Arc<Self>, id: ProfileId) -> anyhow::Result<()> {
        ProfileDetailsWindow::open_root(self.application(), self.coordinator(), self.runtime(), id)
    }

    pub(crate) fn show_trust_entries(self: &Arc<Self>, id: ProfileId) -> anyhow::Result<()> {
        TrustEntriesWindow::open_root(self.application(), self.runtime(), id)
    }
}
