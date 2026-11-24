use crate::app::MacOsApplication;
use crate::app::ui::AppShell;
use crate::app::ui::replace_device_key_window::ReplaceDeviceKeyProps;
use crate::app::ui::replace_device_key_window::ReplaceDeviceKeyWindow;
use crate::app::ui::trust_entries_window::TrustEntriesWindow;
use dispatch2::MainThreadBound;
use keyringd_core::application::ApplicationError;
use keyringd_core::model::ProfileId;
use keyringd_ui_core::key_operation::KeyOperationCoordinator;
use keyringd_ui_core::profile_details_window::ProfileDetailsCommand;
use objc2::{MainThreadMarker, Message, rc::Retained};
use objc2_app_kit::NSWindow;
use std::{cell::RefCell, sync::Arc};

pub(super) struct ProfileDetailsWindowCommandHandler {
    profile_application: Arc<MacOsApplication>,
    runtime: tokio::runtime::Handle,
    interaction: Arc<KeyOperationCoordinator>,
    shell: Retained<AppShell>,
    trust_entries: Option<Arc<MainThreadBound<RefCell<TrustEntriesWindow>>>>,
    replacement: Option<Arc<MainThreadBound<RefCell<ReplaceDeviceKeyWindow>>>>,
    mtm: MainThreadMarker,
}
impl ProfileDetailsWindowCommandHandler {
    pub(super) fn new(
        profile_application: Arc<MacOsApplication>,
        runtime: tokio::runtime::Handle,
        interaction: Arc<KeyOperationCoordinator>,
        shell: Retained<AppShell>,
        mtm: MainThreadMarker,
    ) -> Self {
        Self {
            profile_application,
            runtime,
            interaction,
            shell,
            trust_entries: None,
            replacement: None,
            mtm,
        }
    }
    pub(super) fn handle(
        &mut self,
        command: ProfileDetailsCommand,
        id: &ProfileId,
        window: &NSWindow,
    ) {
        match command {
            ProfileDetailsCommand::OpenTrustEntries => self.open_trust_entries(id, window),
            ProfileDetailsCommand::OpenReplaceDeviceKey => {
                if window.attachedSheet().is_none()
                    && self.open_replace_device_key(id, window).is_err()
                {
                    Self::close(window);
                }
            }
            ProfileDetailsCommand::Close => Self::close(window),
        }
    }
    fn close(window: &NSWindow) {
        if let Some(parent) = window.sheetParent() {
            parent.endSheet(window);
        }
    }
    fn open_trust_entries(&mut self, profile_id: &ProfileId, parent: &NSWindow) {
        let component = self.trust_entries.get_or_insert_with(|| {
            TrustEntriesWindow::mount(
                Arc::clone(&self.profile_application),
                self.runtime.clone(),
                self.mtm,
            )
        });
        component
            .get(self.mtm)
            .borrow_mut()
            .present(profile_id.clone(), parent);
    }

    fn open_replace_device_key(
        &mut self,
        profile_id: &ProfileId,
        parent: &NSWindow,
    ) -> Result<(), ApplicationError> {
        let profile = self.profile_application.profile(profile_id)?;
        let component = self.replacement.get_or_insert_with(|| {
            ReplaceDeviceKeyWindow::mount(
                Arc::clone(&self.profile_application),
                self.runtime.clone(),
                parent.retain(),
                Arc::clone(&self.interaction),
                self.shell.clone(),
                self.mtm,
            )
        });
        component.get(self.mtm).borrow_mut().present(
            ReplaceDeviceKeyProps {
                profile_id: profile_id.clone(),
                profile_name: profile.display_name().to_owned(),
            },
            self.mtm,
        );
        Ok(())
    }
}
