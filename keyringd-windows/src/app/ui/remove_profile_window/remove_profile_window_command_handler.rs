use keyringd_ui_core::toast::ToastMessage;
use std::sync::Arc;

use crate::app::WindowsApplication;

use crate::app::ui::util::AsyncMutation;

use keyringd_core::model::ProfileId;

use crate::app::ui::AuxiliaryWindow;
use keyringd_ui_core::remove_profile_window::RemoveProfileCommand;

#[derive(Clone)]
pub(super) struct RemoveProfileWindowCommandHandler {
    profile_application: Arc<WindowsApplication>,
    id: ProfileId,
    window: AuxiliaryWindow,
    mutation: AsyncMutation<()>,
    name: String,
    notices: tokio::sync::watch::Sender<Option<ToastMessage>>,
}

impl RemoveProfileWindowCommandHandler {
    pub(super) fn new(
        profile_application: Arc<WindowsApplication>,
        id: ProfileId,
        window: AuxiliaryWindow,
        mutation: AsyncMutation<()>,
        name: String,
        notices: tokio::sync::watch::Sender<Option<ToastMessage>>,
    ) -> Self {
        Self {
            profile_application,
            id,
            window,
            mutation,
            name,
            notices,
        }
    }

    pub(super) fn handle(&self, command: RemoveProfileCommand) {
        match command {
            RemoveProfileCommand::Remove => {
                let profile_application = Arc::clone(&self.profile_application);
                let id = self.id.clone();
                let context =
                    crate::system::device_key::CngOperationContext::for_window(self.window.hwnd());
                let notices = self.notices.clone();
                let name = self.name.clone();
                self.mutation
                    .start(
                        keyringd_core::application::ActionId::generate().scope(async move {
                            use keyringd_ui_core::remove_profile_window::RemoveProfileEvent;
                            let event = RemoveProfileEvent::removal_finished(
                                profile_application.remove_profile(&context, &id).await,
                            );
                            if let Some(notice) = event.notice(&name) {
                                notices.send_replace(Some(notice));
                            }
                            let RemoveProfileEvent::RemovalFinished(result) = event else {
                                unreachable!()
                            };
                            result
                        }),
                    );
            }
            RemoveProfileCommand::Close => self.window.close(),
        }
    }
}
