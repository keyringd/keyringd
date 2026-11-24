use super::DeviceKeyProtectionWindowCommandHandler;
use crate::app::ui::util::AsyncMutation;
use crate::{WindowsError, app::WindowsApplication};
use keyringd_core::model::ProfileId;
use keyringd_core::service::DeviceKeyProtectionChange;
use keyringd_ui_core::key_operation::KeyOperationCoordinator;
use keyringd_ui_core::{
    ComponentModel,
    device_key_protection_window::{
        DeviceKeyProtectionEvent, DeviceKeyProtectionResult, DeviceKeyProtectionState,
    },
};
use std::{ops::ControlFlow, sync::Arc};

pub(in crate::app::ui) struct DeviceKeyProtectionWindow;
impl DeviceKeyProtectionWindow {
    pub(in crate::app::ui) fn open(
        profile_application: Arc<WindowsApplication>,
        coordinator: Arc<KeyOperationCoordinator>,
        profile_id: ProfileId,
        change: DeviceKeyProtectionChange,
        owner: usize,
        mutation: AsyncMutation<Option<DeviceKeyProtectionResult>>,
        runtime: tokio::runtime::Handle,
    ) -> Result<(), WindowsError> {
        let handler = DeviceKeyProtectionWindowCommandHandler::new(
            profile_application,
            coordinator,
            profile_id,
            owner,
            runtime,
        )?;
        if !handler.confirm(change)? {
            return Ok(());
        }
        mutation.start(async move {
            let mut state = DeviceKeyProtectionState::ready(change);
            let mut event = DeviceKeyProtectionEvent::Started;
            loop {
                let Some(command) = state.update(event) else {
                    return Ok(None);
                };
                match handler.handle(command).await? {
                    ControlFlow::Continue(next) => event = next,
                    ControlFlow::Break(result) => return Ok(result),
                }
            }
        });
        Ok(())
    }
}
