use keyringd_ui_core::key_operation::KeyOperationCoordinator;
use keyringd_ui_core::text as ui_text;
use std::sync::Arc;

use crate::app::WindowsApplication;

use keyringd_core::model::ProfileId;

use crate::app::ui::AuxiliaryWindow;
use crate::app::ui::replace_device_key_window::ReplaceDeviceKeyWindow;
use crate::app::ui::trust_entries_window::TrustEntriesWindow;
use keyringd_ui_core::profile_details_window::ProfileDetailsCommand;
use keyringd_ui_core::profile_details_window::ProfileDetailsEvent;
use keyringd_ui_core::runtime::UiEventSender;

#[derive(Clone)]
pub(super) struct ProfileDetailsWindowCommandHandler {
    profile_application: Arc<WindowsApplication>,
    coordinator: Arc<KeyOperationCoordinator>,
    runtime: tokio::runtime::Handle,
    id: ProfileId,
    window: AuxiliaryWindow,
    on_toast: keyringd_ui_core::runtime::UiEventTarget<keyringd_ui_core::toast::ToastMessage>,
}

impl ProfileDetailsWindowCommandHandler {
    pub(super) fn new(
        profile_application: Arc<WindowsApplication>,
        coordinator: Arc<KeyOperationCoordinator>,
        runtime: tokio::runtime::Handle,
        id: ProfileId,
        window: AuxiliaryWindow,
        on_toast: keyringd_ui_core::runtime::UiEventTarget<keyringd_ui_core::toast::ToastMessage>,
    ) -> Self {
        Self {
            profile_application,
            coordinator,
            runtime,
            id,
            window,
            on_toast,
        }
    }

    pub(super) fn subscribe_profile_changes(
        &self,
        updates: UiEventSender<ProfileDetailsEvent>,
    ) -> tokio::task::JoinHandle<()> {
        let mut changes = self.profile_application.subscribe_profile_changes();
        let id = self.id.clone();
        self.runtime.spawn(async move {
            loop {
                let profile = changes
                    .borrow_and_update()
                    .iter()
                    .find(|profile| profile.id() == &id)
                    .cloned();
                updates.send(
                    profile.map_or(ProfileDetailsEvent::ProfileRemoved, |profile| {
                        ProfileDetailsEvent::ProfileChanged(Box::new(profile))
                    }),
                );
                if changes.changed().await.is_err() {
                    break;
                }
            }
        })
    }

    pub(super) fn handle(&self, command: ProfileDetailsCommand) -> Option<ProfileDetailsEvent> {
        let opened = match command {
            ProfileDetailsCommand::OpenTrustEntries => TrustEntriesWindow::open_modal(
                Arc::clone(&self.profile_application),
                self.runtime.clone(),
                self.id.clone(),
                self.window.hwnd(),
            ),
            ProfileDetailsCommand::OpenReplaceDeviceKey => ReplaceDeviceKeyWindow::open(
                Arc::clone(&self.profile_application),
                Arc::clone(&self.coordinator),
                self.runtime.clone(),
                self.id.clone(),
                self.window.hwnd(),
                self.on_toast.clone(),
            ),
            ProfileDetailsCommand::Close => {
                self.window.close();
                return None;
            }
        };
        Some(ProfileDetailsEvent::ChildWindowOpened(opened.map_err(
            |error| keyringd_core::util::report_user_error(&error, ui_text::OPEN_WINDOW_FAILED),
        )))
    }
}
