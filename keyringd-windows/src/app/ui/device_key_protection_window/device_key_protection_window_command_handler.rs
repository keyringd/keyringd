use super::DeviceKeyProtectionConfirmationWindow;
use crate::{WindowsError, app::WindowsApplication};
use keyringd_core::model::ProfileId;
use keyringd_core::service::DeviceKeyProtectionChange;
use keyringd_ui_core::device_key_protection_window::{
    DeviceKeyProtectionCommand, DeviceKeyProtectionEvent, DeviceKeyProtectionOutcome,
    DeviceKeyProtectionResult,
};
use keyringd_ui_core::key_operation::KeyOperationCoordinator;
use std::{ops::ControlFlow, sync::Arc};
use windows::Win32::{Foundation::HWND, UI::Input::KeyboardAndMouse::EnableWindow};

pub(super) struct DeviceKeyProtectionWindowCommandHandler {
    application: Arc<WindowsApplication>,
    id: ProfileId,
    name: String,
    owner: usize,
    prompts: crate::app::AuthenticationPromptCommandHandler,
}
impl DeviceKeyProtectionWindowCommandHandler {
    pub(super) fn new(
        application: Arc<WindowsApplication>,
        coordinator: Arc<KeyOperationCoordinator>,
        id: ProfileId,
        owner: usize,
        runtime: tokio::runtime::Handle,
    ) -> Result<Self, WindowsError> {
        let name = application
            .profile(&id)
            .map_err(WindowsError::from)?
            .display_name()
            .to_owned();
        let dispatcher = windows_reactor::WinUIDispatcher::for_current_thread()
            .map_err(|error| WindowsError::Environment(error.to_string().into()))?
            .marshaller();
        let prompts = crate::app::AuthenticationPromptCommandHandler::new(
            dispatcher,
            runtime,
            coordinator.clone(),
        );
        Ok(Self {
            application,
            id,
            name,
            owner,
            prompts,
        })
    }
    pub(super) fn confirm(&self, change: DeviceKeyProtectionChange) -> Result<bool, WindowsError> {
        let accepted =
            DeviceKeyProtectionConfirmationWindow::new(self.owner).confirm(&self.name, change)?;
        if accepted {
            unsafe {
                let _ = EnableWindow(HWND(self.owner as *mut core::ffi::c_void), false);
            }
        }
        Ok(accepted)
    }
    pub(super) async fn handle(
        &self,
        command: DeviceKeyProtectionCommand,
    ) -> Result<ControlFlow<Option<DeviceKeyProtectionResult>, DeviceKeyProtectionEvent>, String>
    {
        match command {
            DeviceKeyProtectionCommand::Change(change) => {
                let coordinator = self.prompts.coordinator.clone();
                let mut flow = coordinator.begin_device_key_protection_change(self.name.clone());
                let flow_id = flow.id();
                let events = flow.take_events();
                let notifications = self
                    .prompts
                    .runtime
                    .spawn(self.prompts.clone().receive_events(flow_id, events));
                let result = flow
                    .run(async {
                        let previous = self.application.profile(&self.id)?;
                        let context =
                            crate::system::device_key::CngOperationContext::for_window(self.owner);
                        self.application
                            .change_device_key_protection(&context, &self.id, change)
                            .await?;
                        let current = self.application.profile(&self.id)?;
                        Ok(DeviceKeyProtectionResult::between(
                            change, &previous, &current,
                        ))
                    })
                    .await;
                let _ = notifications.await;
                let outcome = flow_id
                    .span()
                    .in_scope(|| DeviceKeyProtectionOutcome::from(result));
                Ok(ControlFlow::Continue(DeviceKeyProtectionEvent::Finished(
                    outcome,
                )))
            }
            DeviceKeyProtectionCommand::Notify(result) => Ok(ControlFlow::Break(Some(result))),
            DeviceKeyProtectionCommand::ShowRetry(error) => {
                let owner = self.owner;
                let retry = tokio::task::spawn_blocking(move || {
                    DeviceKeyProtectionConfirmationWindow::new(owner).confirm_retry(&error)
                })
                .await
                .map_err(|error| {
                    keyringd_core::util::report_user_error(
                        &error,
                        keyringd_ui_core::text::OPEN_WINDOW_FAILED,
                    )
                })?
                .map_err(|error| {
                    keyringd_core::util::report_user_error(
                        &error,
                        keyringd_ui_core::text::OPEN_WINDOW_FAILED,
                    )
                })?;
                Ok(ControlFlow::Continue(if retry {
                    DeviceKeyProtectionEvent::Started
                } else {
                    DeviceKeyProtectionEvent::CloseRequested
                }))
            }
            DeviceKeyProtectionCommand::Close => Ok(ControlFlow::Break(None)),
        }
    }
}
