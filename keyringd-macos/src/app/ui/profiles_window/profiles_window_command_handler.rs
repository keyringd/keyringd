use dispatch2::MainThreadBound;
use keyringd_ui_core::device_key_protection_window::DeviceKeyProtectionResult;
use keyringd_ui_core::runtime::{UiEventSender, UiEventTarget};
use keyringd_ui_core::toast::ToastMessage;
use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2_app_kit::NSWindow;
use std::{cell::RefCell, sync::Arc};

use crate::app::MacOsApplication;
use crate::app::ui::AppShell;
use crate::app::ui::device_key_protection_window::DeviceKeyProtectionProps;
use crate::app::ui::device_key_protection_window::DeviceKeyProtectionWindow;
use crate::app::ui::profile_details_window::ProfileDetailsWindow;
use crate::app::ui::profile_setup_window::ProfileSetupWindow;
use crate::app::ui::remove_profile_window::RemoveProfileProps;
use crate::app::ui::remove_profile_window::RemoveProfileWindow;
use crate::app::ui::replace_device_key_window::ReplaceDeviceKeyProps;
use crate::app::ui::replace_device_key_window::ReplaceDeviceKeyWindow;
use crate::app::ui::ssh_connection_test_window::SshConnectionTestProps;
use crate::app::ui::ssh_connection_test_window::SshConnectionTestWindow;
use crate::app::ui::toast::ToastOverlay;
use crate::app::ui::trust_entries_window::TrustEntriesWindow;
use keyringd_ui_core::key_operation::KeyOperationCoordinator;

use super::ProfilesWindowCommand;
use super::ProfilesWindowEvent;

pub(crate) struct ProfilesWindowCommandHandler {
    profile_application: Arc<MacOsApplication>,
    remove: Option<Arc<MainThreadBound<RefCell<RemoveProfileWindow>>>>,
    device_key_local_authentication:
        Option<Arc<MainThreadBound<RefCell<DeviceKeyProtectionWindow>>>>,
    replace_device_key: Option<Arc<MainThreadBound<RefCell<ReplaceDeviceKeyWindow>>>>,
    ssh_connection_test: Option<Arc<MainThreadBound<RefCell<SshConnectionTestWindow>>>>,
    details: Option<Arc<MainThreadBound<RefCell<ProfileDetailsWindow>>>>,
    trust_entries: Option<Arc<MainThreadBound<RefCell<TrustEntriesWindow>>>>,
    setup: Arc<dispatch2::MainThreadBound<std::cell::RefCell<ProfileSetupWindow>>>,
    parent: Retained<NSWindow>,
    shell: Retained<AppShell>,
    events: UiEventSender<ProfilesWindowEvent>,
    runtime: tokio::runtime::Handle,
    socket_path: String,
    home: std::path::PathBuf,
    interaction: Arc<KeyOperationCoordinator>,
    toast: ToastOverlay,
}

impl ProfilesWindowCommandHandler {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        profile_application: Arc<MacOsApplication>,
        runtime: tokio::runtime::Handle,
        socket_path: String,
        home: std::path::PathBuf,
        parent: Retained<NSWindow>,
        setup: Arc<dispatch2::MainThreadBound<std::cell::RefCell<ProfileSetupWindow>>>,
        shell: Retained<AppShell>,
        events: UiEventSender<ProfilesWindowEvent>,
        interaction: Arc<KeyOperationCoordinator>,
        mtm: MainThreadMarker,
    ) -> Self {
        let content = parent.contentView().expect(
            "the initialized Profiles window should have the content view supplied by AppKit",
        );
        let toast = ToastOverlay::new(&content, mtm);
        Self {
            profile_application,
            remove: None,
            device_key_local_authentication: None,
            replace_device_key: None,
            ssh_connection_test: None,
            details: None,
            trust_entries: None,
            setup,
            parent,
            shell,
            events,
            runtime,
            socket_path,
            home,
            interaction,
            toast,
        }
    }

    pub(crate) fn handle(&mut self, command: ProfilesWindowCommand, mtm: MainThreadMarker) {
        match command {
            ProfilesWindowCommand::CheckDeviceKeyAvailability => {
                let application = Arc::clone(&self.profile_application);
                let events = self.events.clone();
                self.runtime.spawn(async move {
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
            }
            ProfilesWindowCommand::AddProfile => {
                self.setup.get(mtm).borrow().show_for(&self.parent);
            }
            ProfilesWindowCommand::ShowDetails(id) => {
                self.shell.present(&self.parent);
                let component = self.details.get_or_insert_with(|| {
                    ProfileDetailsWindow::mount(
                        Arc::clone(&self.profile_application),
                        self.runtime.clone(),
                        Arc::clone(&self.interaction),
                        self.shell.clone(),
                        mtm,
                    )
                });
                component.get(mtm).borrow_mut().present(&id, &self.parent);
            }
            ProfilesWindowCommand::ShowTrustEntries(id) => {
                self.shell.present(&self.parent);
                let parent = self.parent.clone();
                self.show_trust_entries(id, &parent, mtm);
            }
            ProfilesWindowCommand::Remove { id, name } => {
                let component = RemoveProfileWindow::mount(
                    Arc::clone(&self.profile_application),
                    self.runtime.clone(),
                    self.parent.clone(),
                    self.events.map(ProfilesWindowEvent::ToastRequested),
                    mtm,
                );
                component.get(mtm).borrow_mut().present(
                    RemoveProfileProps {
                        profile_id: id,
                        profile_name: name,
                    },
                    mtm,
                );
                self.remove = Some(component);
            }
            ProfilesWindowCommand::ChangeDeviceKeyProtection { id, name, change } => {
                let public_keys = self
                    .trust_entries
                    .get_or_insert_with(|| {
                        TrustEntriesWindow::mount(
                            Arc::clone(&self.profile_application),
                            self.runtime.clone(),
                            mtm,
                        )
                    })
                    .clone();
                let parent = self.parent.clone();
                let events = self.events.clone();
                let on_completed = UiEventTarget::new(move |result: DeviceKeyProtectionResult| {
                    events.send(ProfilesWindowEvent::ToastRequested(ToastMessage::Success(
                        result.notice,
                    )));
                    if let Some(notice) = result.registration_notice {
                        public_keys.get(mtm).borrow_mut().present_with_notice(
                            notice.profile_id,
                            &parent,
                            Some(notice.message),
                        );
                    }
                });
                let component = DeviceKeyProtectionWindow::mount(
                    Arc::clone(&self.profile_application),
                    self.runtime.clone(),
                    self.parent.clone(),
                    Arc::clone(&self.interaction),
                    self.shell.clone(),
                    on_completed,
                    mtm,
                );
                component.get(mtm).borrow_mut().present(
                    DeviceKeyProtectionProps {
                        profile_id: id,
                        profile_name: name,
                        change,
                    },
                    mtm,
                );
                self.device_key_local_authentication = Some(component);
            }
            ProfilesWindowCommand::ReplaceDeviceKey { id, name } => {
                self.show_replace_device_key(id, name, self.parent.clone(), mtm);
            }
            ProfilesWindowCommand::TestConnection(id) => {
                let Ok(profile) = self.profile_application.profile(&id) else {
                    return;
                };
                let component = SshConnectionTestWindow::mount(
                    self.runtime.clone(),
                    Arc::clone(&self.profile_application),
                    self.socket_path.clone(),
                    self.home.clone(),
                    self.parent.clone(),
                    mtm,
                );
                component
                    .get(mtm)
                    .borrow_mut()
                    .present(SshConnectionTestProps { profile }, mtm);
                self.ssh_connection_test = Some(component);
            }
            ProfilesWindowCommand::UpdateProfiles(request) => {
                let application = Arc::clone(&self.profile_application);
                let events = self.events.clone();
                self.runtime.spawn(async move {
                    events.send(request.execute(application).await);
                });
            }
            ProfilesWindowCommand::ShowToast(message) => {
                self.toast.show(message, mtm);
            }
        }
    }
    fn show_trust_entries(
        &mut self,
        id: keyringd_core::model::ProfileId,
        parent: &NSWindow,
        mtm: MainThreadMarker,
    ) {
        let component = self.trust_entries.get_or_insert_with(|| {
            TrustEntriesWindow::mount(
                Arc::clone(&self.profile_application),
                self.runtime.clone(),
                mtm,
            )
        });
        component.get(mtm).borrow_mut().present(id, parent);
    }
    fn show_replace_device_key(
        &mut self,
        id: keyringd_core::model::ProfileId,
        name: String,
        parent: Retained<NSWindow>,
        mtm: MainThreadMarker,
    ) {
        if parent.attachedSheet().is_some() {
            return;
        }
        let component = ReplaceDeviceKeyWindow::mount(
            Arc::clone(&self.profile_application),
            self.runtime.clone(),
            parent,
            Arc::clone(&self.interaction),
            self.shell.clone(),
            mtm,
        );
        component.get(mtm).borrow_mut().present(
            ReplaceDeviceKeyProps {
                profile_id: id,
                profile_name: name,
            },
            mtm,
        );
        self.replace_device_key = Some(component);
    }
}
