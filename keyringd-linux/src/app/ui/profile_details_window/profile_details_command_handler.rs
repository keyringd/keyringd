use std::sync::Arc;

use gtk4::glib::object::Cast;
use gtk4::prelude::GtkWindowExt;

use crate::app::LinuxApplication;
use crate::app::ui::replace_device_key_window::ReplaceDeviceKeyWindow;
use crate::app::ui::trust_entries_window::TrustEntriesWindow;
use keyringd_core::model::ProfileId;
use keyringd_ui_core::key_operation::KeyOperationCoordinator;
use keyringd_ui_core::profile_details_window::ProfileDetailsCommand;
use keyringd_ui_core::replace_device_key_window::ReplaceDeviceKeyProps;

#[derive(Clone)]
pub(super) struct ProfileDetailsCommandHandler {
    application: libadwaita::Application,
    profile_application: Arc<LinuxApplication>,
    prompts: Arc<KeyOperationCoordinator>,
    profile_id: ProfileId,
}

impl ProfileDetailsCommandHandler {
    pub(super) fn new(
        application: libadwaita::Application,
        profile_application: Arc<LinuxApplication>,
        prompts: Arc<KeyOperationCoordinator>,
        profile_id: ProfileId,
    ) -> Self {
        Self {
            application,
            profile_application,
            prompts,
            profile_id,
        }
    }

    pub(super) fn handle(
        &self,
        command: ProfileDetailsCommand,
        window: &libadwaita::MessageDialog,
    ) {
        let owner: gtk4::Window = window.clone().upcast();
        match command {
            ProfileDetailsCommand::OpenTrustEntries => TrustEntriesWindow::present(
                &owner,
                Arc::clone(&self.profile_application),
                self.profile_id.clone(),
            ),
            ProfileDetailsCommand::OpenReplaceDeviceKey => {
                let Ok(profile) = self.profile_application.profile(&self.profile_id) else {
                    window.close();
                    return;
                };
                ReplaceDeviceKeyWindow::present(
                    &owner,
                    ReplaceDeviceKeyProps {
                        profile_id: self.profile_id.clone(),
                        profile_name: profile.display_name().to_owned(),
                    },
                    self.application.clone(),
                    Arc::clone(&self.profile_application),
                    Arc::clone(&self.prompts),
                );
            }
            ProfileDetailsCommand::Close => window.close(),
        }
    }
}
