use keyringd_ui_core::runtime::UiEventTarget;
use std::sync::Arc;

use gtk4::glib::object::Cast;
use gtk4::prelude::ObjectExt;

use crate::app::LinuxApplication;
use crate::app::ui::device_key_protection_window::DeviceKeyProtectionWindow;
use crate::app::ui::profile_details_window::ProfileDetailsWindow;
use crate::app::ui::profile_setup_window::ProfileSetupWindow;
use crate::app::ui::remove_profile_window::RemoveProfileWindow;
use crate::app::ui::ssh_connection_test_window::SshConnectionTestWindow;
use crate::app::ui::trust_entries_window::TrustEntriesWindow;
use crate::system::device_key::{EsapiError, can_grant_tpm_access, grant_tpm_access};
use crate::system::open_ssh::LinuxOpenSshService;
use keyringd_ui_core::device_key_protection_window::{
    DeviceKeyProtectionProps, DeviceKeyProtectionResult,
};
use keyringd_ui_core::key_operation::KeyOperationCoordinator;

use keyringd_core::application::ApplicationError;
use keyringd_ui_core::profiles_window::ProfilesWindowEvent;
use keyringd_ui_core::profiles_window::{DeviceKeyAvailabilityError, ProfilesWindowCommand};
use keyringd_ui_core::remove_profile_window::RemoveProfileProps;
use keyringd_ui_core::ssh_connection_test_window::SshConnectionTestProps;
use keyringd_ui_core::toast::ToastMessage;

/// Executes navigation, platform interaction, and persistence commands emitted by ProfilesWindowState.
pub(super) struct ProfilesWindowCommandHandler {
    application: libadwaita::Application,
    profile_application: Arc<LinuxApplication>,
    open_ssh: Arc<LinuxOpenSshService>,
    prompts: Arc<KeyOperationCoordinator>,
    on_toast: UiEventTarget<ToastMessage>,
    events: keyringd_ui_core::runtime::UiEventSender<ProfilesWindowEvent>,
}

impl ProfilesWindowCommandHandler {
    pub(super) fn new(
        application: libadwaita::Application,
        profile_application: Arc<LinuxApplication>,
        open_ssh: Arc<LinuxOpenSshService>,
        prompts: Arc<KeyOperationCoordinator>,
        on_toast: UiEventTarget<ToastMessage>,
        events: keyringd_ui_core::runtime::UiEventSender<ProfilesWindowEvent>,
    ) -> Self {
        Self {
            application,
            profile_application,
            open_ssh,
            prompts,
            on_toast,
            events,
        }
    }

    pub(super) fn request_tpm_access(&self, on_finished: impl FnOnce() + 'static) {
        let task = tokio::spawn(grant_tpm_access());
        let events = self.events.clone();
        gtk4::glib::MainContext::ref_thread_default().spawn_local(async move {
            match task.await.map_err(std::io::Error::other).and_then(|r| r) {
                Ok(true) => events.send(ProfilesWindowEvent::DeviceKeyAvailabilityChecked(Err(
                    DeviceKeyAvailabilityError::Unavailable(
                        "TPM access was granted. Sign out and back in to use it.".into(),
                    ),
                ))),
                Ok(false) => {}
                Err(error) => events.send(ProfilesWindowEvent::ToastRequested(
                    ToastMessage::Error(keyringd_core::util::report_user_error(
                        &error,
                        "Could not grant TPM access. Ask your administrator for help.",
                    )),
                )),
            }
            on_finished();
        });
    }

    pub(super) fn handle(&self, command: ProfilesWindowCommand, parent: &gtk4::Window) {
        match command {
            ProfilesWindowCommand::CheckDeviceKeyAvailability => {
                let application = Arc::clone(&self.profile_application);
                let events = self.events.clone();
                tokio::spawn(async move {
                    let result = application
                        .check_device_key_availability()
                        .await
                        .map_err(|error| {
                            if let ApplicationError::DeviceKeyUnavailable(source) = &error
                                && matches!(
                                    source.downcast_ref::<EsapiError>(),
                                    Some(EsapiError::AccessDenied(_))
                                )
                            {
                                let can_grant = can_grant_tpm_access();
                                let message = if can_grant {
                                    "TPM access is required to create a Profile."
                                } else {
                                    "TPM access is required to create a Profile. Ask your administrator for access to /dev/tpmrm0."
                                };
                                return DeviceKeyAvailabilityError::AccessDenied {
                                    message: error.report_user_error(message),
                                    can_grant,
                                };
                            }
                            DeviceKeyAvailabilityError::Unavailable(
                                error.report_user_error(&error.user_message()),
                            )
                        });
                    events.send(ProfilesWindowEvent::DeviceKeyAvailabilityChecked(result));
                });
            }
            ProfilesWindowCommand::AddProfile => {
                let Some(parent) = parent.downcast_ref::<libadwaita::ApplicationWindow>() else {
                    return;
                };
                ProfileSetupWindow::present(
                    parent,
                    self.application.clone(),
                    Arc::clone(&self.profile_application),
                    Arc::clone(&self.prompts),
                );
            }
            ProfilesWindowCommand::ShowDetails(id) => ProfileDetailsWindow::present(
                parent,
                id,
                self.application.clone(),
                Arc::clone(&self.profile_application),
                Arc::clone(&self.prompts),
            ),
            ProfilesWindowCommand::ShowTrustEntries(id) => {
                TrustEntriesWindow::present(parent, Arc::clone(&self.profile_application), id)
            }
            ProfilesWindowCommand::TestConnection(id) => {
                let Ok(profile) = self.profile_application.profile(&id) else {
                    return;
                };
                SshConnectionTestWindow::present(
                    parent,
                    SshConnectionTestProps { profile },
                    Arc::clone(&self.open_ssh),
                    Arc::clone(&self.profile_application),
                );
            }
            ProfilesWindowCommand::ChangeDeviceKeyProtection { id, name, change } => {
                let profile_application = Arc::clone(&self.profile_application);
                let events = self.events.clone();
                let on_toast = UiEventTarget::new(move |toast| {
                    events.send(ProfilesWindowEvent::ToastRequested(toast))
                });
                let parent_for_result = parent.downgrade();
                let on_completed = UiEventTarget::new(move |result: DeviceKeyProtectionResult| {
                    on_toast.send(ToastMessage::Success(result.notice));
                    if let (Some(notice), Some(parent)) =
                        (result.registration_notice, parent_for_result.upgrade())
                    {
                        TrustEntriesWindow::present_with_notice(
                            &parent,
                            Arc::clone(&profile_application),
                            notice.profile_id,
                            Some(notice.message),
                        );
                    }
                });
                DeviceKeyProtectionWindow::present(
                    parent,
                    DeviceKeyProtectionProps {
                        profile_id: id,
                        profile_name: name,
                        change,
                    },
                    self.application.clone(),
                    Arc::clone(&self.profile_application),
                    Arc::clone(&self.prompts),
                    on_completed,
                )
            }
            ProfilesWindowCommand::Remove { id, name } => RemoveProfileWindow::present(
                parent,
                RemoveProfileProps {
                    profile_id: id,
                    profile_name: name,
                },
                Arc::clone(&self.profile_application),
                {
                    let events = self.events.clone();
                    UiEventTarget::new(move |toast| {
                        events.send(ProfilesWindowEvent::ToastRequested(toast))
                    })
                },
            ),
            ProfilesWindowCommand::UpdateProfiles(mutation) => {
                let application = Arc::clone(&self.profile_application);
                let events = self.events.clone();
                tokio::spawn(async move {
                    events.send(mutation.execute(application).await);
                });
            }
            ProfilesWindowCommand::ReplaceDeviceKey { id, name } => {
                crate::app::ui::replace_device_key_window::ReplaceDeviceKeyWindow::present(
                    parent,
                    keyringd_ui_core::replace_device_key_window::ReplaceDeviceKeyProps {
                        profile_id: id,
                        profile_name: name,
                    },
                    self.application.clone(),
                    Arc::clone(&self.profile_application),
                    Arc::clone(&self.prompts),
                )
            }
            ProfilesWindowCommand::ShowToast(toast) => self.on_toast.send(toast),
        }
    }
}
