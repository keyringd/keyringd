use super::DeviceKeyProtectionWindow;
use crate::app::{AuthenticationPromptCommandHandler, LinuxApplication};
use gtk4::prelude::WidgetExt;
use keyringd_core::model::ProfileId;
use keyringd_core::service::DeviceKeyProtectionChange;
use keyringd_ui_core::device_key_protection_window::{
    DeviceKeyProtectionCommand, DeviceKeyProtectionEvent, DeviceKeyProtectionOutcome,
    DeviceKeyProtectionProps, DeviceKeyProtectionResult,
};
use keyringd_ui_core::key_operation::KeyOperationCoordinator;
use keyringd_ui_core::runtime::UiEventTarget;
use std::sync::Arc;

pub(super) struct DeviceKeyProtectionCommandHandler {
    parent: gtk4::Window,
    props: DeviceKeyProtectionProps,
    application: libadwaita::Application,
    profile_application: Arc<LinuxApplication>,
    prompts: Arc<KeyOperationCoordinator>,
    on_completed: UiEventTarget<DeviceKeyProtectionResult>,
}
impl DeviceKeyProtectionCommandHandler {
    pub(super) fn new(
        parent: gtk4::Window,
        props: DeviceKeyProtectionProps,
        application: libadwaita::Application,
        profile_application: Arc<LinuxApplication>,
        prompts: Arc<KeyOperationCoordinator>,
        on_completed: UiEventTarget<DeviceKeyProtectionResult>,
    ) -> Self {
        Self {
            parent,
            props,
            application,
            profile_application,
            prompts,
            on_completed,
        }
    }
    pub(super) async fn handle(
        &self,
        command: DeviceKeyProtectionCommand,
    ) -> Option<DeviceKeyProtectionEvent> {
        match command {
            DeviceKeyProtectionCommand::Change(change) => self.authorize(change).await,
            DeviceKeyProtectionCommand::Notify(result) => {
                self.on_completed.send(result);
                None
            }
            DeviceKeyProtectionCommand::ShowRetry(error) => Some(
                if self.parent.is_visible()
                    && DeviceKeyProtectionWindow::confirm_retry(&self.parent, &error).await
                {
                    DeviceKeyProtectionEvent::Started
                } else {
                    DeviceKeyProtectionEvent::CloseRequested
                },
            ),
            DeviceKeyProtectionCommand::Close => None,
        }
    }
    async fn authorize(
        &self,
        change: DeviceKeyProtectionChange,
    ) -> Option<DeviceKeyProtectionEvent> {
        let profile_application = Arc::clone(&self.profile_application);
        let profile_id = self.props.profile_id.clone();
        let action = self
            .prompts
            .begin_device_key_protection_change(self.props.profile_name.clone());
        AuthenticationPromptCommandHandler::run_action(
            self.application.clone(),
            self.parent.clone(),
            Arc::clone(&self.prompts),
            action,
            Self::execute(profile_application, profile_id, change),
        )
        .await
    }

    async fn execute(
        profile_application: Arc<LinuxApplication>,
        profile_id: ProfileId,
        change: DeviceKeyProtectionChange,
    ) -> DeviceKeyProtectionEvent {
        let operation = async {
            let previous = profile_application.profile(&profile_id)?;
            profile_application
                .change_device_key_protection(&(), &profile_id, change)
                .await?;
            let current = profile_application.profile(&profile_id)?;
            Ok(DeviceKeyProtectionResult::between(
                change, &previous, &current,
            ))
        };
        DeviceKeyProtectionEvent::Finished(DeviceKeyProtectionOutcome::from(operation.await))
    }
}
