use crate::app::WindowsApplication;
use keyringd_core::model::ProfileId;
use keyringd_ui_core::runtime::{UiEventSender, UiEventTarget};
use keyringd_ui_core::trust_entries_window::{TrustEntriesEvent, TrustEntriesState};
use std::sync::Arc;

use crate::app::ui::AuxiliaryWindow;
use keyringd_ui_core::toast::ToastMessage;
use keyringd_ui_core::trust_entries_window::TrustEntriesCommand;

#[derive(Clone)]
pub(super) struct TrustEntriesWindowCommandHandler {
    profile_application: Arc<WindowsApplication>,
    runtime: tokio::runtime::Handle,
    profile_id: ProfileId,
    window: AuxiliaryWindow,
    on_toast: UiEventTarget<ToastMessage>,
}

impl TrustEntriesWindowCommandHandler {
    pub(super) fn new(
        profile_application: Arc<WindowsApplication>,
        runtime: tokio::runtime::Handle,
        profile_id: ProfileId,
        window: AuxiliaryWindow,
        on_toast: UiEventTarget<ToastMessage>,
    ) -> Self {
        Self {
            profile_application,
            runtime,
            profile_id,
            window,
            on_toast,
        }
    }

    pub(super) fn subscribe_profile_changes(
        &self,
        updates: UiEventSender<TrustEntriesEvent>,
    ) -> tokio::task::JoinHandle<()> {
        let mut changes = self.profile_application.subscribe_profile_changes();
        let id = self.profile_id.clone();
        self.runtime.spawn(async move {
            loop {
                let entries = changes
                    .borrow_and_update()
                    .iter()
                    .find(|profile| profile.id() == &id)
                    .map(TrustEntriesState::from_profile);
                updates.send(entries.map_or(
                    TrustEntriesEvent::ProfileRemoved,
                    TrustEntriesEvent::EntriesChanged,
                ));
                if changes.changed().await.is_err() {
                    break;
                }
            }
        })
    }

    pub(super) fn handle(&self, command: TrustEntriesCommand) {
        match command {
            TrustEntriesCommand::Copy {
                value,
                success_message,
            } => {
                let message = match crate::system::clipboard::copy(&value) {
                    Ok(()) => ToastMessage::Success(success_message.to_owned()),
                    Err(error) => ToastMessage::Error(keyringd_core::util::report_user_error(
                        &error,
                        keyringd_ui_core::text::COPY_FAILED,
                    )),
                };
                self.on_toast.send(message);
            }
            TrustEntriesCommand::Close => {
                self.window.close();
            }
        }
    }
}
