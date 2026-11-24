use crate::app::ui::MainThreadEventReceiverExt as _;

use keyringd_ui_core::runtime::UiEventSender;
use keyringd_ui_core::text as ui_text;
use std::sync::Arc;

use block2::RcBlock;
use dispatch2::MainThreadBound;
use keyringd_core::model::{Profile, ProfileId};
use objc2::{MainThreadMarker, MainThreadOnly, sel};
use objc2_app_kit::{
    NSAccessibility, NSBackingStoreType, NSButton, NSModalResponse, NSTextField,
    NSUserInterfaceLayoutOrientation, NSView, NSWindow, NSWindowStyleMask,
};
use objc2_foundation::{NSPoint, NSRect, NSSize, NSString};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

use super::ProfileDetailsEvent;
use super::ProfileDetailsState;
use super::ProfileDetailsWindowActionTarget;
use super::ProfileDetailsWindowCommandHandler;
use crate::app::MacOsApplication;
use crate::app::ui::AppShell;
use crate::app::ui::ComponentModel;
use crate::app::ui::Layout;
use crate::app::ui::UiEventReceiver;
use keyringd_ui_core::key_operation::KeyOperationCoordinator;
use objc2::rc::Retained;

pub(crate) struct ProfileDetailsWindow {
    state: Option<ProfileDetailsState>,
    content: Retained<NSView>,
    heading: Retained<NSTextField>,
    status_value: Retained<NSTextField>,
    device_format_value: Retained<NSTextField>,
    device_key_value: Retained<NSTextField>,
    _targets: Vec<Retained<ProfileDetailsWindowActionTarget>>,
    window: Retained<NSWindow>,
    presented: Rc<Cell<bool>>,
    command_handler: ProfileDetailsWindowCommandHandler,
    profiles: tokio::sync::watch::Receiver<Vec<Profile>>,
}

impl ProfileDetailsWindow {
    pub(crate) fn mount(
        profile_application: Arc<MacOsApplication>,
        runtime_handle: tokio::runtime::Handle,
        interaction: Arc<KeyOperationCoordinator>,
        shell: Retained<AppShell>,
        mtm: MainThreadMarker,
    ) -> Arc<MainThreadBound<RefCell<Self>>> {
        let (events_tx, events_rx) = UiEventSender::channel();
        let window = unsafe {
            NSWindow::initWithContentRect_styleMask_backing_defer(
                NSWindow::alloc(mtm),
                NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(600.0, 564.0)),
                NSWindowStyleMask::Titled,
                NSBackingStoreType::Buffered,
                false,
            )
        };
        window.setTitle(&NSString::from_str(ui_text::PROFILE_DETAILS_WINDOW_TITLE));
        unsafe { window.setReleasedWhenClosed(false) };
        let content = window
            .contentView()
            .expect("the initialized Profile Details window should have the content view supplied by AppKit");
        let heading = Layout::label("", 22.0, true, mtm);
        heading.setAccessibilityIdentifier(Some(&NSString::from_str("profile-details.heading")));
        let status_value = Layout::label("", 13.0, false, mtm);
        status_value
            .setAccessibilityIdentifier(Some(&NSString::from_str("profile-details.status")));
        let device_format_value = Layout::label("", 13.0, false, mtm);
        device_format_value.setAccessibilityIdentifier(Some(&NSString::from_str(
            "profile-details.device-ssh-format",
        )));
        let device_key_value = Layout::label("", 13.0, false, mtm);
        device_key_value.setAccessibilityIdentifier(Some(&NSString::from_str(
            "profile-details.local-authentication",
        )));
        let status = Self::row(ui_text::PROFILE_DETAILS_STATUS, &status_value, mtm);
        let format = Self::row(
            ui_text::PROFILE_DETAILS_SIGNATURE_FORMAT,
            &device_format_value,
            mtm,
        );
        let protection = Self::row(
            ui_text::PROFILE_DETAILS_LOCAL_AUTHENTICATION,
            &device_key_value,
            mtm,
        );
        let summary = Layout::group(&[&status, &format, &protection], false, mtm);
        let access_heading = Layout::label(ui_text::PROFILE_DETAILS_ACCESS, 14.0, true, mtm);
        let access_label = Layout::label(ui_text::PROFILE_DETAILS_REGISTRATIONS, 13.0, false, mtm);
        Layout::flexible_width(&access_label);
        let (trust_button, trust_target) = Self::button(
            ui_text::TRUST_ENTRIES_MENU_ACTION,
            "profile-details.trust-entries",
            ProfileDetailsEvent::TrustEntriesRequested,
            events_tx.clone(),
            mtm,
        );
        let access_row = Layout::stack(
            &[&access_label, &trust_button],
            NSUserInterfaceLayoutOrientation::Horizontal,
            16.0,
            mtm,
        );
        let access = Layout::group(&[&access_row], false, mtm);
        let advanced_heading = Layout::label(ui_text::PROFILE_DETAILS_ADVANCED, 14.0, true, mtm);
        let replace_heading =
            Layout::label(ui_text::REPLACE_DEVICE_KEY_WINDOW_TITLE, 13.0, true, mtm);
        let replace_description = Layout::label(
            &keyringd_ui_core::profile_details_window::replacement_description("Secure Enclave"),
            12.0,
            false,
            mtm,
        );
        let replace_copy = Layout::stack(
            &[&replace_heading, &replace_description],
            NSUserInterfaceLayoutOrientation::Vertical,
            4.0,
            mtm,
        );
        let (replace_button, replace_target) = Self::button(
            ui_text::PROFILE_DETAILS_REPLACE,
            "profile-details.replace-device-key",
            ProfileDetailsEvent::ReplaceDeviceKeyRequested,
            events_tx.clone(),
            mtm,
        );
        let replace_row = Layout::stack(
            &[&replace_copy, &replace_button],
            NSUserInterfaceLayoutOrientation::Horizontal,
            16.0,
            mtm,
        );
        let advanced = Layout::group(&[&replace_row], false, mtm);
        let (close, close_target) = Self::button(
            ui_text::CLOSE,
            "profile-details.close",
            ProfileDetailsEvent::CloseRequested,
            events_tx,
            mtm,
        );
        close.setKeyEquivalent(&NSString::from_str("\u{1b}"));
        let actions = Layout::actions(&[&close], mtm);
        let stack = Layout::stack(
            &[
                &heading,
                &summary,
                &access_heading,
                &access,
                &advanced_heading,
                &advanced,
                &actions,
            ],
            NSUserInterfaceLayoutOrientation::Vertical,
            16.0,
            mtm,
        );
        content.addSubview(&stack);
        Layout::pin(&stack, &content, 28.0, 28.0, 20.0, 28.0);
        Layout::width(&stack, 544.0);
        let command_handler = ProfileDetailsWindowCommandHandler::new(
            Arc::clone(&profile_application),
            runtime_handle.clone(),
            interaction,
            shell,
            mtm,
        );
        let snapshot = profile_application.subscribe_profile_changes();
        let (_, changes) = UiEventReceiver::from_tokio_watch(&runtime_handle, snapshot.clone());
        let component = Arc::new(MainThreadBound::new(
            RefCell::new(Self {
                state: None,
                content,
                heading,
                status_value,
                device_format_value,
                device_key_value,
                _targets: vec![trust_target, replace_target, close_target],
                window,
                presented: Rc::new(Cell::new(false)),
                command_handler,
                profiles: snapshot,
            }),
            mtm,
        ));
        let weak = Arc::downgrade(&component);
        events_rx.spawn_on_main(&runtime_handle, move |event, mtm| {
            if let Some(component) = weak.upgrade() {
                component.get(mtm).borrow_mut().dispatch(event);
            }
        });
        let weak = Arc::downgrade(&component);
        changes.spawn_on_main(&runtime_handle, move |_, mtm| {
            if let Some(component) = weak.upgrade() {
                component.get(mtm).borrow_mut().profiles_changed();
            }
        });
        component
    }

    pub(crate) fn present(&mut self, id: &ProfileId, parent: &NSWindow) {
        if self.presented.get() || parent.attachedSheet().is_some() {
            return;
        }
        let profile = self
            .profiles
            .borrow()
            .iter()
            .find(|profile| profile.id() == id)
            .cloned();
        let Some(profile) = profile else { return };
        let state = ProfileDetailsState::new(&profile);
        self.render(&state);
        self.state = Some(state);
        self.presented.set(true);
        let presented = Rc::clone(&self.presented);
        let window = self.window.clone();
        let completion = RcBlock::new(move |_: NSModalResponse| {
            window.orderOut(None);
            presented.set(false);
        });
        parent.beginSheet_completionHandler(&self.window, Some(&completion));
    }

    fn profiles_changed(&mut self) {
        if !self.window.isVisible() {
            return;
        }
        let Some(state) = &self.state else {
            return;
        };
        let id = state.profile_id().clone();
        let event = self
            .profiles
            .borrow()
            .iter()
            .find(|profile| profile.id() == &id)
            .map(|profile| ProfileDetailsEvent::ProfileChanged(Box::new(profile.clone())))
            .unwrap_or(ProfileDetailsEvent::ProfileRemoved);
        self.dispatch(event);
    }

    fn dispatch(&mut self, event: ProfileDetailsEvent) {
        let Some(state) = &mut self.state else {
            return;
        };
        let profile_id = state.profile_id().clone();
        let command = state.update(event);
        if let Some(state) = &self.state {
            self.render(state);
        }
        if let Some(command) = command {
            self.command_handler
                .handle(command, &profile_id, &self.window);
        }
    }

    fn render(&self, state: &ProfileDetailsState) {
        self.heading
            .setStringValue(&NSString::from_str(state.heading()));
        self.status_value
            .setStringValue(&NSString::from_str(state.status()));
        self.device_format_value
            .setStringValue(&NSString::from_str(state.device_key_signature_algorithm()));
        self.device_key_value
            .setStringValue(&NSString::from_str(state.device_key_local_authentication()));
        Layout::fit_window(&self.content);
    }

    fn row(
        title: &str,
        value: &NSView,
        mtm: MainThreadMarker,
    ) -> Retained<objc2_app_kit::NSStackView> {
        let label = Layout::label(title, 13.0, true, mtm);
        let row = Layout::stack(
            &[&label, value],
            NSUserInterfaceLayoutOrientation::Horizontal,
            16.0,
            mtm,
        );
        label
            .widthAnchor()
            .constraintEqualToAnchor(&value.widthAnchor())
            .setActive(true);
        row
    }

    fn button(
        title: &str,
        id: &str,
        event: ProfileDetailsEvent,
        events: UiEventSender<ProfileDetailsEvent>,
        mtm: MainThreadMarker,
    ) -> (
        Retained<NSButton>,
        Retained<ProfileDetailsWindowActionTarget>,
    ) {
        let target = ProfileDetailsWindowActionTarget::new(mtm, events, event);
        let button = unsafe {
            NSButton::buttonWithTitle_target_action(
                &NSString::from_str(title),
                Some(&target),
                Some(sel!(performAction:)),
                mtm,
            )
        };
        button.setAccessibilityIdentifier(Some(&NSString::from_str(id)));
        // Preserve each action's intrinsic width when its description wraps.
        button.setContentCompressionResistancePriority_forOrientation(
            objc2_app_kit::NSLayoutPriorityRequired,
            objc2_app_kit::NSLayoutConstraintOrientation::Horizontal,
        );
        (button, target)
    }
}
