use keyringd_ui_core::replace_device_key_window::ReplaceDeviceKeyCommand;
use std::sync::Arc;

use crate::app::WindowsApplication;

use crate::app::ui::util::AsyncMutation;

use keyringd_core::model::ProfileId;

use crate::app::ui::AuxiliaryWindow;
use crate::app::ui::trust_entries_window::TrustEntriesWindow;
use keyringd_ui_core::replace_device_key_window::DeviceKeyReplacementOutcome;

#[derive(Clone)]
pub(super) struct ReplaceDeviceKeyWindowCommandHandler {
    pub(super) profile_application: Arc<WindowsApplication>,
    pub(super) id: ProfileId,
    pub(super) window: AuxiliaryWindow,
    pub(super) parent: usize,
    pub(super) on_toast:
        keyringd_ui_core::runtime::UiEventTarget<keyringd_ui_core::toast::ToastMessage>,
    pub(super) mutation: AsyncMutation<DeviceKeyReplacementOutcome>,
    pub(super) prompts: crate::app::AuthenticationPromptCommandHandler,
}

impl ReplaceDeviceKeyWindowCommandHandler {
    pub(super) fn show(&self) {
        self.window.show();
    }

    pub(super) fn handle(&self, command: ReplaceDeviceKeyCommand) {
        match command {
            ReplaceDeviceKeyCommand::Replace => {
                let application = Arc::clone(&self.profile_application);
                let id = self.id.clone();
                let owner = self.window.hwnd();
                self.window.hide();
                let profile_name = self
                    .profile_application
                    .profile(&id)
                    .map(|profile| profile.display_name().to_owned())
                    .unwrap_or_default();
                let prompts = self.prompts.clone();
                self.mutation.start(async move {
                    Ok(
                        Self::execute_replacement(application, prompts, id, profile_name, owner)
                            .await,
                    )
                });
            }
            ReplaceDeviceKeyCommand::OpenTrustEntriesAndClose => {
                self.window.close();
                if let Err(error) = TrustEntriesWindow::open_modal(
                    Arc::clone(&self.profile_application),
                    self.prompts.runtime.clone(),
                    self.id.clone(),
                    self.parent,
                ) {
                    self.on_toast
                        .send(keyringd_ui_core::toast::ToastMessage::Error(
                            keyringd_core::util::report_user_error(
                                &error,
                                keyringd_ui_core::text::OPEN_PUBLIC_KEYS_FAILED,
                            ),
                        ));
                }
            }
            ReplaceDeviceKeyCommand::Close => {
                self.window.close();
            }
        }
    }
    async fn execute_replacement(
        application: Arc<WindowsApplication>,
        prompts: crate::app::AuthenticationPromptCommandHandler,
        id: ProfileId,
        profile_name: String,
        owner: usize,
    ) -> DeviceKeyReplacementOutcome {
        let coordinator = prompts.coordinator.clone();
        let mut flow = coordinator.begin_device_key_replacement(profile_name);
        let flow_id = flow.id();
        let events = flow.take_events();
        let notifications = prompts
            .runtime
            .clone()
            .spawn(prompts.receive_events(flow_id, events));
        let context = crate::system::device_key::CngOperationContext::for_window(owner);
        let result = flow
            .run(application.replace_device_key(&context, &id))
            .await;

        let _ = notifications.await;
        flow_id
            .span()
            .in_scope(|| DeviceKeyReplacementOutcome::from(result))
    }
}
