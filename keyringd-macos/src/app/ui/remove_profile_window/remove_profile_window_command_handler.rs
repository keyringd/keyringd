use crate::app::MacOsApplication;
use keyringd_ui_core::remove_profile_window::{
    RemoveProfileCommand, RemoveProfileEvent, RemoveProfileProps,
};
use keyringd_ui_core::runtime::UiEventSender;
use keyringd_ui_core::toast::ToastMessage;
use objc2::rc::Retained;
use objc2_app_kit::NSWindow;
use std::sync::Arc;

pub(super) struct RemoveProfileWindowCommandHandler {
    profile_application: Arc<MacOsApplication>,
    runtime: tokio::runtime::Handle,
    events: UiEventSender<RemoveProfileEvent>,
    profile_events: UiEventSender<ToastMessage>,
    window: Option<Retained<NSWindow>>,
}

impl RemoveProfileWindowCommandHandler {
    pub(super) fn new(
        profile_application: Arc<MacOsApplication>,
        runtime: tokio::runtime::Handle,
        events: UiEventSender<RemoveProfileEvent>,
        profile_events: UiEventSender<ToastMessage>,
    ) -> Self {
        Self {
            profile_application,
            runtime,
            events,
            profile_events,
            window: None,
        }
    }

    pub(super) fn set_window(&mut self, window: Retained<NSWindow>) {
        self.window = Some(window);
    }

    pub(super) fn is_active(&self) -> bool {
        self.window.is_some()
    }

    pub(super) fn handle(&mut self, command: RemoveProfileCommand, props: &RemoveProfileProps) {
        match command {
            RemoveProfileCommand::Remove => {
                let application = Arc::clone(&self.profile_application);
                let id = props.profile_id.clone();
                let name = props.profile_name.clone();
                let events = self.events.clone();
                let profile_events = self.profile_events.clone();
                // Deletion and its notice outlive the confirmation sheet.
                self.runtime
                    .spawn(
                        keyringd_core::application::ActionId::generate().scope(async move {
                            let event = RemoveProfileEvent::removal_finished(
                                application.remove_profile(&(), &id).await,
                            );
                            if let Some(notice) = event.notice(&name) {
                                profile_events.send(notice);
                            }
                            events.send(event);
                        }),
                    );
            }
            RemoveProfileCommand::Close => {
                if let Some(window) = self.window.take()
                    && let Some(parent) = window.sheetParent()
                {
                    parent.endSheet(&window);
                }
            }
        }
    }
}
