use keyringd_ui_core::device_key_protection_window::DeviceKeyProtectionResult;

use keyringd_ui_core::runtime::UiEventTarget;
use keyringd_ui_core::text as ui_text;
use keyringd_ui_core::toast::ToastMessage;
use std::sync::Arc;

use crate::app::ui::util::AsyncMutation;

use windows::Win32::Foundation::HWND;

use windows::Win32::UI::Input::KeyboardAndMouse::EnableWindow;

use windows::Win32::UI::WindowsAndMessaging::{IsWindow, SetForegroundWindow};

use windows_reactor::Updater;

use crate::app::ui::ComponentModel;
use crate::app::ui::reduce;

use crate::app::ui::device_key_protection_window::DeviceKeyProtectionWindow;
use crate::app::ui::profile_details_window::ProfileDetailsWindow;
use crate::app::ui::profile_setup_window::ProfileSetupWindow;
use crate::app::ui::profiles_window::ProfilesWindow;
use crate::app::ui::remove_profile_window::RemoveProfileWindow;
use crate::app::ui::ssh_connection_test_window::SshConnectionTestWindow;
use crate::app::ui::trust_entries_window::TrustEntriesWindow;
use keyringd_ui_core::profiles_window::ProfilesWindowCommand;
use keyringd_ui_core::profiles_window::ProfilesWindowEvent;
use keyringd_ui_core::profiles_window::ProfilesWindowState;

/// Executes the commands emitted by [`ProfilesWindowState`].
pub(super) struct ProfilesWindowCommandHandler {
    window: Arc<ProfilesWindow>,
    profile_operation: AsyncMutation<Option<DeviceKeyProtectionResult>>,
    on_toast: UiEventTarget<ToastMessage>,
    events: keyringd_ui_core::runtime::UiEventSender<ProfilesWindowEvent>,
}

impl ProfilesWindowCommandHandler {
    pub(super) fn new(
        window: Arc<ProfilesWindow>,
        profile_operation: AsyncMutation<Option<DeviceKeyProtectionResult>>,
        on_toast: UiEventTarget<ToastMessage>,
        events: keyringd_ui_core::runtime::UiEventSender<ProfilesWindowEvent>,
    ) -> Self {
        Self {
            window,
            profile_operation,
            on_toast,
            events,
        }
    }

    pub(super) fn event_handler(
        self,
        set_model: Updater<ProfilesWindowState>,
    ) -> UiEventTarget<ProfilesWindowEvent> {
        UiEventTarget::new(move |event| {
            let event = match event {
                ProfilesWindowEvent::ProfilesChanged(_) => {
                    ProfilesWindowEvent::ProfilesChanged(self.window.application().profiles())
                }
                event => event,
            };
            if matches!(event, ProfilesWindowEvent::MutationFinished { .. }) {
                reduce(
                    &set_model,
                    ProfilesWindowEvent::ProfilesChanged(self.window.application().profiles()),
                );
            }
            let (_, command) = reduce(&set_model, event);
            let Some(command) = command else { return };
            if let ProfilesWindowCommand::ShowToast(toast) = command {
                self.on_toast.send(toast);
                return;
            }
            let result = self.handle(command);
            Self::apply_result(&set_model, result);
        })
    }

    fn handle(&self, command: ProfilesWindowCommand) -> std::result::Result<(), String> {
        match command {
            ProfilesWindowCommand::CheckDeviceKeyAvailability => {
                let application = self.window.application();
                let events = self.events.clone();
                self.window.runtime().spawn(async move {
                    let result = application
                        .check_device_key_availability()
                        .await
                        .map_err(|error| {
                            keyringd_ui_core::profiles_window::DeviceKeyAvailabilityError::Unavailable(
                                error.report_user_error(&error.user_message()),
                            )
                        });
                    events.send(ProfilesWindowEvent::DeviceKeyAvailabilityChecked(result));
                });
                Ok(())
            }
            ProfilesWindowCommand::AddProfile => ProfileSetupWindow::new(
                self.window.application(),
                self.window.runtime(),
                self.window.window_handle(),
                self.window.coordinator(),
            )
            .open()
            .map_err(|error| {
                keyringd_core::util::report_user_error(&error, ui_text::OPEN_WINDOW_FAILED)
            }),
            ProfilesWindowCommand::ShowTrustEntries(id) => TrustEntriesWindow::open_modal(
                self.window.application(),
                self.window.runtime(),
                id,
                self.window.window_handle(),
            )
            .map_err(|error| {
                keyringd_core::util::report_user_error(&error, ui_text::OPEN_WINDOW_FAILED)
            }),
            ProfilesWindowCommand::ShowDetails(id) => ProfileDetailsWindow::open_modal(
                self.window.application(),
                self.window.coordinator(),
                self.window.runtime(),
                id,
                self.window.window_handle(),
            )
            .map_err(|error| {
                keyringd_core::util::report_user_error(&error, ui_text::OPEN_WINDOW_FAILED)
            }),
            ProfilesWindowCommand::TestConnection(id) => SshConnectionTestWindow::open(
                self.window.application(),
                self.window.runtime(),
                id,
                self.window.window_handle(),
                #[cfg(feature = "test-support")]
                self.window.test_open_ssh.clone(),
            )
            .map_err(|error| {
                keyringd_core::util::report_user_error(&error, ui_text::OPEN_WINDOW_FAILED)
            }),
            ProfilesWindowCommand::ChangeDeviceKeyProtection { id, change, .. } => {
                DeviceKeyProtectionWindow::open(
                    self.window.application(),
                    self.window.coordinator(),
                    id,
                    change,
                    self.window.window_handle(),
                    self.profile_operation.clone(),
                    self.window.runtime(),
                )
                .map_err(|error| error.report_user_error(&error.user_message()))
            }
            ProfilesWindowCommand::Remove { id, .. } => RemoveProfileWindow::open(
                self.window.application(),
                self.window.runtime(),
                id,
                self.window.window_handle(),
                self.window.removal_notices.clone(),
            )
            .map_err(|error| {
                keyringd_core::util::report_user_error(&error, ui_text::OPEN_WINDOW_FAILED)
            }),
            ProfilesWindowCommand::UpdateProfiles(request) => {
                let application = self.window.application();
                let events = self.events.clone();
                self.window.runtime().spawn(async move {
                    events.send(request.execute(application).await);
                });
                Ok(())
            }
            ProfilesWindowCommand::ReplaceDeviceKey { id, .. } => {
                crate::app::ui::replace_device_key_window::ReplaceDeviceKeyWindow::open(
                    self.window.application(),
                    self.window.coordinator(),
                    self.window.runtime(),
                    id,
                    self.window.window_handle(),
                    self.on_toast.clone(),
                )
                .map_err(|error| {
                    keyringd_core::util::report_user_error(&error, ui_text::OPEN_WINDOW_FAILED)
                })
            }
            ProfilesWindowCommand::ShowToast(toast) => {
                self.on_toast.send(toast);
                Ok(())
            }
        }
    }

    pub(super) fn complete_profile_operation(
        set_model: &Updater<ProfilesWindowState>,
        owner_value: usize,
        result: std::result::Result<(), String>,
    ) {
        let owner = HWND(owner_value as *mut core::ffi::c_void);
        if owner_value != 0 && unsafe { IsWindow(Some(owner)) }.as_bool() {
            unsafe {
                let _ = EnableWindow(owner, true);
                let _ = SetForegroundWindow(owner);
            }
        }
        Self::apply_result(set_model, result);
    }

    pub(super) fn apply_result(
        set_model: &Updater<ProfilesWindowState>,
        result: std::result::Result<(), String>,
    ) {
        set_model.call(move |mut model| {
            model.update(ProfilesWindowEvent::OperationFinished(result));
            model
        });
    }
}
