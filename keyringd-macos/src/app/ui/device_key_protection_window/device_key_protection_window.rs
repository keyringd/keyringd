use super::DeviceKeyProtectionProps;
use super::DeviceKeyProtectionState;
use super::DeviceKeyProtectionWindowCommandHandler;
use crate::app::ui::AppShell;
use crate::app::ui::MainThreadEventReceiverExt as _;
use crate::app::{MacOsApplication, ui::ComponentRuntime};
use block2::RcBlock;
use dispatch2::MainThreadBound;
use keyringd_ui_core::device_key_protection_window::DeviceKeyProtectionEvent;
use keyringd_ui_core::key_operation::KeyOperationCoordinator;
use keyringd_ui_core::runtime::{UiEventSender, UiEventTarget};
use keyringd_ui_core::text as ui_text;
use objc2::{MainThreadMarker, rc::Retained};
use objc2_app_kit::{NSAccessibility, NSAlert, NSAlertFirstButtonReturn, NSWindow};
use objc2_foundation::NSString;
use std::{cell::RefCell, sync::Arc};

pub(crate) struct DeviceKeyProtectionWindow {
    handler: DeviceKeyProtectionWindowCommandHandler,
    parent: Retained<NSWindow>,
    events: UiEventSender<DeviceKeyProtectionEvent>,
    state: Option<ComponentRuntime<DeviceKeyProtectionState>>,
    props: Option<DeviceKeyProtectionProps>,
    mtm: MainThreadMarker,
    on_completed:
        UiEventTarget<keyringd_ui_core::device_key_protection_window::DeviceKeyProtectionResult>,
}
impl DeviceKeyProtectionWindow {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn mount(
        profile_application: Arc<MacOsApplication>,
        runtime: tokio::runtime::Handle,
        parent: Retained<NSWindow>,
        interaction: Arc<KeyOperationCoordinator>,
        shell: Retained<AppShell>,
        on_completed: UiEventTarget<
            keyringd_ui_core::device_key_protection_window::DeviceKeyProtectionResult,
        >,
        mtm: MainThreadMarker,
    ) -> Arc<MainThreadBound<RefCell<Self>>> {
        let (events, receiver) = UiEventSender::channel();
        let handler = DeviceKeyProtectionWindowCommandHandler::new(
            profile_application,
            runtime.clone(),
            parent.clone(),
            events.clone(),
            interaction,
            shell,
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
                on_completed,
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
    pub(crate) fn present(&mut self, props: DeviceKeyProtectionProps, _mtm: MainThreadMarker) {
        if self.parent.attachedSheet().is_some()
            || self
                .state
                .as_ref()
                .is_some_and(|state| state.state().is_authorizing())
        {
            return;
        }

        self.state = Some(ComponentRuntime::new(DeviceKeyProtectionState::ready(
            props.change,
        )));
        self.props = Some(props.clone());
        let window = Self::present_alert(
            &self.parent,
            self.events.clone(),
            self.mtm,
            &keyringd_ui_core::device_key_protection_window::heading(
                &props.profile_name,
                props.change,
            ),
            &keyringd_ui_core::device_key_protection_window::confirmation(
                "A replacement Secure Enclave key will be created with the selected local authentication settings, then your Passkey will authorize it. Public Keys will show any updated server registration entries.",
            ),
            ui_text::CONTINUE,
        );
        self.handler.set_window(window);
    }
    fn dispatch(&mut self, event: DeviceKeyProtectionEvent) {
        let Some(state) = &self.state else { return };
        let command = state.dispatch(event);
        if let Some(command) = command
            && let Some(props) = &self.props
        {
            self.handler.handle(command, props, &self.on_completed);
        }
        if !self.handler.is_active() {
            self.state = None;
            self.props = None;
        }
    }
    pub(super) fn present_alert(
        parent: &NSWindow,
        events: UiEventSender<DeviceKeyProtectionEvent>,
        mtm: MainThreadMarker,
        title: &str,
        detail: &str,
        action: &str,
    ) -> Retained<NSWindow> {
        let alert = NSAlert::new(mtm);
        alert.setMessageText(&NSString::from_str(title));
        alert.setInformativeText(&NSString::from_str(detail));
        alert
            .window()
            .setAccessibilityIdentifier(Some(&NSString::from_str("device-key-protection.prompt")));
        alert
            .addButtonWithTitle(&NSString::from_str(action))
            .setAccessibilityIdentifier(Some(&NSString::from_str("device-key-protection.confirm")));
        alert
            .addButtonWithTitle(&NSString::from_str(ui_text::CANCEL))
            .setAccessibilityIdentifier(Some(&NSString::from_str("device-key-protection.cancel")));
        let completion = RcBlock::new(move |response| {
            let event = if response == NSAlertFirstButtonReturn {
                DeviceKeyProtectionEvent::Started
            } else {
                DeviceKeyProtectionEvent::CloseRequested
            };
            events.send(event);
        });
        alert.beginSheetModalForWindow_completionHandler(parent, Some(&completion));
        alert.window()
    }
}
