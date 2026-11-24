use super::DeviceKeyProtectionWindow;
use crate::app::ui::{AppShell, MainThreadEventReceiverExt as _};
use crate::app::{MacOsApplication, MacOsKeyOperationSession};
use dispatch2::MainThreadBound;
use keyringd_core::model::ProfileId;
use keyringd_core::service::DeviceKeyProtectionChange;
use keyringd_ui_core::device_key_protection_window::{
    DeviceKeyProtectionCommand, DeviceKeyProtectionEvent, DeviceKeyProtectionOutcome,
    DeviceKeyProtectionProps, DeviceKeyProtectionResult,
};
use keyringd_ui_core::key_operation::KeyOperationCoordinator;
use keyringd_ui_core::runtime::{UiEventSender, UiEventTarget};
use keyringd_ui_core::text as ui_text;
use objc2::{MainThreadMarker, rc::Retained};
use objc2_app_kit::NSWindow;
use std::{cell::RefCell, sync::Arc};

pub(super) struct DeviceKeyProtectionWindowCommandHandler {
    profile_application: Arc<MacOsApplication>,
    runtime: tokio::runtime::Handle,
    parent: Retained<NSWindow>,
    events: UiEventSender<DeviceKeyProtectionEvent>,
    interaction: Arc<KeyOperationCoordinator>,
    shell: Retained<AppShell>,
    key_operation: Option<Arc<MainThreadBound<RefCell<Option<MacOsKeyOperationSession>>>>>,
    window: Option<Retained<NSWindow>>,
    mtm: MainThreadMarker,
}
impl DeviceKeyProtectionWindowCommandHandler {
    pub(super) fn new(
        profile_application: Arc<MacOsApplication>,
        runtime: tokio::runtime::Handle,
        parent: Retained<NSWindow>,
        events: UiEventSender<DeviceKeyProtectionEvent>,
        interaction: Arc<KeyOperationCoordinator>,
        shell: Retained<AppShell>,
        mtm: MainThreadMarker,
    ) -> Self {
        Self {
            profile_application,
            runtime,
            parent,
            events,
            interaction,
            shell,
            mtm,
            window: None,
            key_operation: None,
        }
    }
    pub(super) fn set_window(&mut self, window: Retained<NSWindow>) {
        self.window = Some(window);
    }
    pub(super) fn is_active(&self) -> bool {
        self.window.is_some()
    }
    pub(super) fn handle(
        &mut self,
        command: DeviceKeyProtectionCommand,
        props: &DeviceKeyProtectionProps,
        on_completed: &UiEventTarget<DeviceKeyProtectionResult>,
    ) {
        match command {
            DeviceKeyProtectionCommand::Change(change) => self.start(props, change),
            DeviceKeyProtectionCommand::Notify(result) => {
                self.close();
                on_completed.send(result);
            }
            DeviceKeyProtectionCommand::ShowRetry(error) => {
                self.window = Some(DeviceKeyProtectionWindow::present_alert(
                    &self.parent,
                    self.events.clone(),
                    self.mtm,
                    ui_text::DEVICE_KEY_PROTECTION_ERROR_TITLE,
                    &error,
                    ui_text::DEVICE_KEY_PROTECTION_RETRY,
                ));
            }
            DeviceKeyProtectionCommand::Close => self.close(),
        }
    }
    fn close(&mut self) {
        if let Some(session) = self.key_operation.take() {
            session.get(self.mtm).borrow_mut().take();
        }
        if let Some(window) = self.window.take()
            && let Some(parent) = window.sheetParent()
        {
            parent.endSheet(&window);
        }
    }
    fn start(&mut self, props: &DeviceKeyProtectionProps, change: DeviceKeyProtectionChange) {
        let mut flow = self
            .interaction
            .begin_device_key_protection_change(props.profile_name.clone());
        let session = MacOsKeyOperationSession::start(
            &self.runtime,
            self.mtm,
            Arc::clone(&self.interaction),
            &mut flow,
            self.shell.clone(),
        );
        self.key_operation = Some(Arc::clone(&session));
        let application = Arc::clone(&self.profile_application);
        let profile_id = props.profile_id.clone();
        let (finished, receiver) = UiEventSender::channel();
        self.runtime.spawn(async move {
            let event = flow
                .run(Self::execute(application, profile_id, change))
                .await;

            finished.send(event);
        });
        let events = self.events.clone();
        receiver.spawn_on_main(&self.runtime, move |event, mtm| {
            session.get(mtm).borrow_mut().take();
            events.send(event);
        });
    }
    async fn execute(
        profile_application: Arc<MacOsApplication>,
        id: ProfileId,
        change: DeviceKeyProtectionChange,
    ) -> DeviceKeyProtectionEvent {
        let operation = async {
            let previous = profile_application.profile(&id)?;
            profile_application
                .change_device_key_protection(&(), &id, change)
                .await?;
            let current = profile_application.profile(&id)?;
            Ok(DeviceKeyProtectionResult::between(
                change, &previous, &current,
            ))
        };
        let result = operation.await;
        DeviceKeyProtectionEvent::Finished(DeviceKeyProtectionOutcome::from(result))
    }
}
