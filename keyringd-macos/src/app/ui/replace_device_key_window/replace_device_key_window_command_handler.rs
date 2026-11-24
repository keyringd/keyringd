use crate::app::ui::{
    AppShell, MainThreadEventReceiverExt as _, trust_entries_window::TrustEntriesWindow,
};
use crate::app::{MacOsApplication, MacOsKeyOperationSession};
use dispatch2::MainThreadBound;
use keyringd_ui_core::key_operation::KeyOperationCoordinator;
use keyringd_ui_core::replace_device_key_window::{
    DeviceKeyReplacementOutcome, ReplaceDeviceKeyCommand, ReplaceDeviceKeyEvent,
    ReplaceDeviceKeyProps,
};
use keyringd_ui_core::runtime::UiEventSender;
use objc2::{MainThreadMarker, rc::Retained};
use objc2_app_kit::NSWindow;
use std::{cell::RefCell, sync::Arc};

pub(super) struct ReplaceDeviceKeyWindowCommandHandler {
    profile_application: Arc<MacOsApplication>,
    runtime: tokio::runtime::Handle,
    events: UiEventSender<ReplaceDeviceKeyEvent>,
    parent: Retained<NSWindow>,
    window: Option<Retained<NSWindow>>,
    trust_entries: Arc<MainThreadBound<RefCell<TrustEntriesWindow>>>,
    interaction: Arc<KeyOperationCoordinator>,
    shell: Retained<AppShell>,
    key_operation: Option<Arc<MainThreadBound<RefCell<Option<MacOsKeyOperationSession>>>>>,
    mtm: MainThreadMarker,
}

impl ReplaceDeviceKeyWindowCommandHandler {
    pub(super) fn new(
        profile_application: Arc<MacOsApplication>,
        runtime: tokio::runtime::Handle,
        parent: Retained<NSWindow>,
        interaction: Arc<KeyOperationCoordinator>,
        shell: Retained<AppShell>,
        events: UiEventSender<ReplaceDeviceKeyEvent>,
        mtm: MainThreadMarker,
    ) -> Self {
        let trust_entries =
            TrustEntriesWindow::mount(Arc::clone(&profile_application), runtime.clone(), mtm);
        Self {
            profile_application,
            runtime,
            events,
            parent,
            window: None,
            trust_entries,
            interaction,
            shell,
            key_operation: None,
            mtm,
        }
    }

    pub(super) fn set_window(&mut self, window: Retained<NSWindow>) {
        self.window = Some(window);
    }

    pub(super) fn handle(
        &mut self,
        command: ReplaceDeviceKeyCommand,
        props: &ReplaceDeviceKeyProps,
    ) {
        match command {
            ReplaceDeviceKeyCommand::Replace => self.start(props),
            ReplaceDeviceKeyCommand::OpenTrustEntriesAndClose => {
                self.close();
                self.trust_entries
                    .get(self.mtm)
                    .borrow_mut()
                    .present(props.profile_id.clone(), &self.parent);
            }
            ReplaceDeviceKeyCommand::Close => self.close(),
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

    fn start(&mut self, props: &ReplaceDeviceKeyProps) {
        let mut flow = self
            .interaction
            .begin_device_key_replacement(props.profile_name.clone());
        let id = flow.id();
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
            let outcome = flow
                .run(application.replace_device_key(&(), &profile_id))
                .await;

            finished.send(id.span().in_scope(|| {
                ReplaceDeviceKeyEvent::ReplacementFinished(DeviceKeyReplacementOutcome::from(
                    outcome,
                ))
            }));
        });
        let events = self.events.clone();
        receiver.spawn_on_main(&self.runtime, move |event, mtm| {
            session.get(mtm).borrow_mut().take();
            events.send(event);
        });
    }
}
