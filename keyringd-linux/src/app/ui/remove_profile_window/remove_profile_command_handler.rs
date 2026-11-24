use crate::app::LinuxApplication;
use gtk4::glib::WeakRef;
use gtk4::prelude::GtkWindowExt;
use keyringd_ui_core::remove_profile_window::{
    RemoveProfileCommand, RemoveProfileEvent, RemoveProfileProps,
};
use keyringd_ui_core::runtime::{UiEventSender, UiEventTarget};
use keyringd_ui_core::toast::ToastMessage;
use std::{rc::Rc, sync::Arc};

#[derive(Clone)]
pub(super) struct RemoveProfileCommandHandler {
    props: RemoveProfileProps,
    profile_application: Arc<LinuxApplication>,
    dialog: WeakRef<libadwaita::MessageDialog>,
    on_toast: UiEventTarget<ToastMessage>,
}

impl RemoveProfileCommandHandler {
    pub(super) fn new(
        props: RemoveProfileProps,
        profile_application: Arc<LinuxApplication>,
        dialog: WeakRef<libadwaita::MessageDialog>,
        on_toast: UiEventTarget<ToastMessage>,
    ) -> Self {
        Self {
            props,
            profile_application,
            dialog,
            on_toast,
        }
    }

    pub(super) fn handle(
        &self,
        command: RemoveProfileCommand,
        on_event: Rc<dyn Fn(RemoveProfileEvent)>,
    ) {
        match command {
            RemoveProfileCommand::Remove => {
                let application = Arc::clone(&self.profile_application);
                let id = self.props.profile_id.clone();
                let name = self.props.profile_name.clone();
                let on_toast = self.on_toast.clone();
                let (finished, mut receiver) = UiEventSender::channel();
                // Deletion completes and reports its outcome even if the dialog is closed.
                tokio::spawn(
                    keyringd_core::application::ActionId::generate().scope(async move {
                        let result = application.remove_profile(&(), &id).await;
                        if let Err(error) = &result {
                            error.log_failure(&format!("failed to remove profile {}", id.as_str()));
                        }
                        finished.send(RemoveProfileEvent::removal_finished(result));
                    }),
                );
                gtk4::glib::MainContext::ref_thread_default().spawn_local(async move {
                    if let Some(event) = receiver.receive().await {
                        if let Some(notice) = event.notice(&name) {
                            on_toast.send(notice);
                        }
                        on_event(event);
                    }
                });
            }
            RemoveProfileCommand::Close => {
                if let Some(dialog) = self.dialog.upgrade() {
                    dialog.close();
                }
            }
        }
    }
}
