use crate::app::ui::trust_entries_window::TrustEntriesWindow;
use crate::app::{AuthenticationPromptCommandHandler, LinuxApplication};
use gtk4::prelude::GtkWindowExt;
use keyringd_ui_core::key_operation::KeyOperationCoordinator;
use keyringd_ui_core::replace_device_key_window::{
    DeviceKeyReplacementOutcome, ReplaceDeviceKeyCommand, ReplaceDeviceKeyEvent,
    ReplaceDeviceKeyProps,
};
use std::{rc::Rc, sync::Arc};

#[derive(Clone)]
pub(super) struct ReplaceDeviceKeyCommandHandler {
    parent: gtk4::Window,
    props: ReplaceDeviceKeyProps,
    application: libadwaita::Application,
    profile_application: Arc<LinuxApplication>,
    prompts: Arc<KeyOperationCoordinator>,
}

impl ReplaceDeviceKeyCommandHandler {
    pub(super) fn new(
        parent: gtk4::Window,
        props: ReplaceDeviceKeyProps,
        application: libadwaita::Application,
        profile_application: Arc<LinuxApplication>,
        prompts: Arc<KeyOperationCoordinator>,
    ) -> Self {
        Self {
            parent,
            props,
            application,
            profile_application,
            prompts,
        }
    }

    pub(super) fn handle(
        &self,
        command: ReplaceDeviceKeyCommand,
        dialog: &libadwaita::MessageDialog,
        on_event: Rc<dyn Fn(ReplaceDeviceKeyEvent)>,
    ) {
        match command {
            ReplaceDeviceKeyCommand::Replace => {
                dialog.close();
                self.start(on_event);
            }
            ReplaceDeviceKeyCommand::OpenTrustEntriesAndClose => {
                dialog.close();
                TrustEntriesWindow::present(
                    &self.parent,
                    Arc::clone(&self.profile_application),
                    self.props.profile_id.clone(),
                );
            }
            ReplaceDeviceKeyCommand::Close => dialog.close(),
        }
    }

    fn start(&self, on_event: Rc<dyn Fn(ReplaceDeviceKeyEvent)>) {
        let parent = self.parent.clone();
        let application = self.application.clone();
        let prompts = Arc::clone(&self.prompts);
        let profile_application = Arc::clone(&self.profile_application);
        let profile_id = self.props.profile_id.clone();
        let action = prompts.begin_device_key_replacement(self.props.profile_name.clone());
        gtk4::glib::MainContext::ref_thread_default().spawn_local(async move {
            let event = AuthenticationPromptCommandHandler::run_action(
                application,
                parent,
                prompts,
                action,
                async move {
                    ReplaceDeviceKeyEvent::ReplacementFinished(DeviceKeyReplacementOutcome::from(
                        profile_application
                            .replace_device_key(&(), &profile_id)
                            .await,
                    ))
                },
            )
            .await;
            if let Some(event) = event {
                on_event(event);
            }
        });
    }
}
