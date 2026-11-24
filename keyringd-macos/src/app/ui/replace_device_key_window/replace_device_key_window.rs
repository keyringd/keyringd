use super::ReplaceDeviceKeyProps;
use super::ReplaceDeviceKeyWindowCommandHandler;
use crate::app::MacOsApplication;
use crate::app::ui::AppShell;
use crate::app::ui::MainThreadEventReceiverExt as _;
use block2::RcBlock;
use dispatch2::MainThreadBound;
use keyringd_ui_core::ComponentModel;
use keyringd_ui_core::key_operation::KeyOperationCoordinator;
use keyringd_ui_core::replace_device_key_window::{ReplaceDeviceKeyEvent, ReplaceDeviceKeyState};
use keyringd_ui_core::runtime::UiEventSender;
use objc2::{MainThreadMarker, rc::Retained};
use objc2_app_kit::{NSAccessibility, NSAlert, NSAlertFirstButtonReturn, NSWindow};
use objc2_foundation::NSString;
use std::{cell::RefCell, sync::Arc};

pub(crate) struct ReplaceDeviceKeyWindow {
    handler: ReplaceDeviceKeyWindowCommandHandler,
    parent: Retained<NSWindow>,
    events: UiEventSender<ReplaceDeviceKeyEvent>,
    state: Option<ReplaceDeviceKeyState>,
    props: Option<ReplaceDeviceKeyProps>,
    mtm: MainThreadMarker,
}
impl ReplaceDeviceKeyWindow {
    pub(crate) fn mount(
        profile_application: Arc<MacOsApplication>,
        runtime: tokio::runtime::Handle,
        parent: Retained<NSWindow>,
        interaction: Arc<KeyOperationCoordinator>,
        shell: Retained<AppShell>,
        mtm: MainThreadMarker,
    ) -> Arc<MainThreadBound<RefCell<Self>>> {
        let (events, receiver) = UiEventSender::channel();
        let handler = ReplaceDeviceKeyWindowCommandHandler::new(
            profile_application,
            runtime.clone(),
            parent.clone(),
            interaction,
            shell,
            events.clone(),
            mtm,
        );
        let component = Arc::new(MainThreadBound::new(
            RefCell::new(Self {
                handler,
                parent,
                events,
                state: None,
                props: None,
                mtm,
            }),
            mtm,
        ));
        let weak = Arc::downgrade(&component);
        receiver.spawn_on_main(&runtime, move |event, mtm| {
            let Some(component) = weak.upgrade() else {
                return;
            };
            component.get(mtm).borrow_mut().dispatch(event);
        });
        component
    }
    pub(crate) fn present(&mut self, props: ReplaceDeviceKeyProps, _mtm: MainThreadMarker) {
        if self.parent.attachedSheet().is_some()
            || self.state.as_ref().is_some_and(|state| state.is_busy())
        {
            return;
        }
        self.state = Some(ReplaceDeviceKeyState::default());
        self.props = Some(props);
        self.present_alert();
    }
    fn dispatch(&mut self, event: ReplaceDeviceKeyEvent) {
        let Some(state) = self.state.as_mut() else {
            return;
        };
        let finished =
            matches!(event, ReplaceDeviceKeyEvent::ReplacementFinished(_)) && state.is_busy();
        let command = state.update(event);
        if let Some(command) = command
            && let Some(props) = &self.props
        {
            self.handler.handle(command, props);
        }
        if matches!(&self.state, Some(ReplaceDeviceKeyState::Closed)) {
            self.state = None;
            self.props = None;
        }
        if finished {
            self.present_alert();
        }
    }
    fn present_alert(&mut self) {
        let Some(state) = &self.state else {
            return;
        };
        let alert = NSAlert::new(self.mtm);
        alert.setMessageText(&NSString::from_str(state.heading()));
        alert.setInformativeText(&NSString::from_str(state.message()));
        alert
            .window()
            .setAccessibilityIdentifier(Some(&NSString::from_str("replace-device-key.prompt")));
        alert
            .addButtonWithTitle(&NSString::from_str(state.primary_label()))
            .setAccessibilityIdentifier(Some(&NSString::from_str(if state.is_complete() {
                "replace-device-key.trust-entries"
            } else {
                "replace-device-key.confirm"
            })));
        alert
            .addButtonWithTitle(&NSString::from_str(state.close_label()))
            .setAccessibilityIdentifier(Some(&NSString::from_str(if state.is_complete() {
                "replace-device-key.close"
            } else {
                "replace-device-key.cancel"
            })));
        let events = self.events.clone();
        let completion = RcBlock::new(move |response| {
            let event = if response == NSAlertFirstButtonReturn {
                ReplaceDeviceKeyEvent::PrimaryRequested
            } else {
                ReplaceDeviceKeyEvent::CloseRequested
            };
            events.send(event);
        });
        self.handler.set_window(alert.window());
        alert.beginSheetModalForWindow_completionHandler(&self.parent, Some(&completion));
    }
}
