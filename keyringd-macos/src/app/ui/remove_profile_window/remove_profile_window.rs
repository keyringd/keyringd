use super::RemoveProfileProps;
use super::RemoveProfileWindowCommandHandler;
use crate::app::MacOsApplication;
use crate::app::ui::MainThreadEventReceiverExt as _;
use block2::RcBlock;
use dispatch2::MainThreadBound;
use keyringd_ui_core::ComponentModel;
use keyringd_ui_core::remove_profile_window::{RemoveProfileEvent, RemoveProfileState};
use keyringd_ui_core::runtime::UiEventSender;
use keyringd_ui_core::text as ui_text;
use keyringd_ui_core::toast::ToastMessage;
use objc2::{MainThreadMarker, rc::Retained};
use objc2_app_kit::{NSAccessibility, NSAlert, NSAlertFirstButtonReturn, NSWindow};
use objc2_foundation::NSString;
use std::{cell::RefCell, sync::Arc};

pub(crate) struct RemoveProfileWindow {
    handler: RemoveProfileWindowCommandHandler,
    parent: Retained<NSWindow>,
    events: UiEventSender<RemoveProfileEvent>,
    state: Option<RemoveProfileState>,
    props: Option<RemoveProfileProps>,
    mtm: MainThreadMarker,
}

impl RemoveProfileWindow {
    pub(crate) fn mount(
        profile_application: Arc<MacOsApplication>,
        runtime: tokio::runtime::Handle,
        parent: Retained<NSWindow>,
        profile_events: UiEventSender<ToastMessage>,
        mtm: MainThreadMarker,
    ) -> Arc<MainThreadBound<RefCell<Self>>> {
        let (events, receiver) = UiEventSender::channel();
        let component = Arc::new(MainThreadBound::new(
            RefCell::new(Self {
                handler: RemoveProfileWindowCommandHandler::new(
                    profile_application,
                    runtime.clone(),
                    events.clone(),
                    profile_events,
                ),
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

    pub(crate) fn present(&mut self, props: RemoveProfileProps, _mtm: MainThreadMarker) {
        self.state = Some(RemoveProfileState::default());
        self.props = Some(props);
        self.present_alert();
    }

    fn dispatch(&mut self, event: RemoveProfileEvent) {
        let Some(state) = self.state.as_mut() else {
            return;
        };
        let finishing =
            state.is_removing() && matches!(&event, RemoveProfileEvent::RemovalFinished(_));
        let command = state.update(event);
        let show_error = finishing && state.error().is_some();
        if let Some(command) = command
            && let Some(props) = &self.props
        {
            self.handler.handle(command, props);
        }
        if !self.handler.is_active() {
            self.state = None;
            self.props = None;
        }
        if show_error {
            self.present_alert();
        }
    }

    fn present_alert(&mut self) {
        let (Some(state), Some(props)) = (&self.state, &self.props) else {
            return;
        };
        let alert = NSAlert::new(self.mtm);
        alert.setMessageText(&NSString::from_str(&state.heading(&props.profile_name)));
        alert.setInformativeText(&NSString::from_str(state.message()));
        alert
            .window()
            .setAccessibilityIdentifier(Some(&NSString::from_str("remove-profile.prompt")));
        alert
            .addButtonWithTitle(&NSString::from_str(state.remove_label()))
            .setAccessibilityIdentifier(Some(&NSString::from_str("remove-profile.confirm")));
        alert
            .addButtonWithTitle(&NSString::from_str(ui_text::CANCEL))
            .setAccessibilityIdentifier(Some(&NSString::from_str("remove-profile.cancel")));
        let events = self.events.clone();
        let completion = RcBlock::new(move |response| {
            let event = if response == NSAlertFirstButtonReturn {
                RemoveProfileEvent::RemovalRequested
            } else {
                RemoveProfileEvent::CloseRequested
            };
            events.send(event);
        });
        self.handler.set_window(alert.window());
        alert.beginSheetModalForWindow_completionHandler(&self.parent, Some(&completion));
    }
}
