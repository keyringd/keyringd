use super::SshConnectionTestActionTarget;
use super::SshConnectionTestCommandHandler;
use super::SshConnectionTestEvent;
use super::SshConnectionTestProps;
use super::SshConnectionTestState;
use super::SshHostFieldDelegate;
use crate::app::ui::MainThreadEventReceiverExt as _;
use crate::app::ui::{ComponentRuntime, Layout};
use dispatch2::MainThreadBound;
use keyringd_core::model::ProfileSshIdentity;
use keyringd_core::model::ProfileSshIdentityKind;
use keyringd_ui_core::runtime::UiEventSender;
use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2::{MainThreadMarker, MainThreadOnly, sel};
use objc2_app_kit::{
    NSAccessibility, NSBackingStoreType, NSButton, NSColor, NSControlStateValueOn, NSStackView,
    NSTextField, NSUserInterfaceLayoutOrientation, NSWindow, NSWindowStyleMask,
};
use objc2_foundation::{NSPoint, NSRect, NSSize, NSString};

use keyringd_ui_core::text as ui_text;
use std::{cell::RefCell, sync::Arc};
pub(crate) struct SshConnectionTestWindow {
    runtime: tokio::runtime::Handle,
    application: Arc<crate::app::MacOsApplication>,
    open_ssh: Arc<crate::system::open_ssh::MacOsOpenSshService>,
    events: UiEventSender<SshConnectionTestEvent>,
    parent: Retained<NSWindow>,
    sheet: Retained<NSWindow>,
    input: Retained<NSTextField>,
    identity_choices: Retained<NSStackView>,
    identity_buttons: Vec<Retained<NSButton>>,
    status: Retained<NSTextField>,
    submit: Retained<NSButton>,
    cancel: Retained<NSButton>,
    state: Option<ComponentRuntime<SshConnectionTestState>>,
    _static_targets: Vec<Retained<SshConnectionTestActionTarget>>,
    identity_targets: Vec<Retained<SshConnectionTestActionTarget>>,
    _host_delegate: Retained<SshHostFieldDelegate>,
    command_handler: Option<SshConnectionTestCommandHandler>,
}
impl SshConnectionTestWindow {
    pub(crate) fn mount(
        runtime: tokio::runtime::Handle,
        application: Arc<crate::app::MacOsApplication>,
        socket_path: String,
        home: std::path::PathBuf,
        parent: Retained<NSWindow>,
        mtm: MainThreadMarker,
    ) -> Arc<MainThreadBound<RefCell<Self>>> {
        let (events, receiver) = UiEventSender::channel();
        let sheet = unsafe {
            NSWindow::initWithContentRect_styleMask_backing_defer(
                NSWindow::alloc(mtm),
                NSRect::new(NSPoint::new(0., 0.), NSSize::new(520., 520.)),
                NSWindowStyleMask::Titled,
                NSBackingStoreType::Buffered,
                false,
            )
        };
        sheet.setTitle(&NSString::from_str(
            ui_text::SSH_CONNECTION_TEST_WINDOW_TITLE,
        ));
        let content = sheet
            .contentView()
            .expect("the initialized SSH connection test sheet should have the content view supplied by AppKit");
        let heading = Layout::label(ui_text::SSH_CONNECTION_TEST_WINDOW_TITLE, 20.0, true, mtm);
        let intro = Layout::label(ui_text::SSH_CONNECTION_TEST_DESCRIPTION, 12.0, false, mtm);
        let host_label = Layout::label(ui_text::SSH_CONNECTION_TEST_HOST_LABEL, 12.0, true, mtm);
        let input = NSTextField::new(mtm);
        input.setPlaceholderString(Some(&NSString::from_str(
            ui_text::SSH_CONNECTION_TEST_HOST_PLACEHOLDER,
        )));
        input.setAccessibilityIdentifier(Some(&NSString::from_str("connection-test.host")));
        let identity_label =
            Layout::label(ui_text::SSH_CONNECTION_TEST_IDENTITY_LABEL, 12.0, true, mtm);
        let identity_choices =
            Layout::stack(&[], NSUserInterfaceLayoutOrientation::Vertical, 12.0, mtm);
        let status = Layout::label("", 12.0, false, mtm);
        status.setAccessibilityIdentifier(Some(&NSString::from_str("connection-test.status")));
        status.setSelectable(true);
        let host_delegate = SshHostFieldDelegate::new(mtm, input.clone(), events.clone());
        unsafe {
            input.setDelegate(Some(ProtocolObject::from_ref(&*host_delegate)));
        }
        let submit_target = SshConnectionTestActionTarget::new(
            mtm,
            events.clone(),
            SshConnectionTestEvent::TestRequested,
        );
        let cancel_target = SshConnectionTestActionTarget::new(
            mtm,
            events.clone(),
            SshConnectionTestEvent::CloseRequested,
        );
        let submit = unsafe {
            NSButton::buttonWithTitle_target_action(
                &NSString::from_str(ui_text::SSH_CONNECTION_TEST_TEST),
                Some(&submit_target),
                Some(sel!(performAction:)),
                mtm,
            )
        };
        submit.setAccessibilityIdentifier(Some(&NSString::from_str("connection-test.test")));
        submit.setKeyEquivalent(&NSString::from_str("\r"));
        let cancel = unsafe {
            NSButton::buttonWithTitle_target_action(
                &NSString::from_str(ui_text::CANCEL),
                Some(&cancel_target),
                Some(sel!(performAction:)),
                mtm,
            )
        };
        cancel.setAccessibilityIdentifier(Some(&NSString::from_str("connection-test.cancel")));
        cancel.setKeyEquivalent(&NSString::from_str("\u{1b}"));
        let actions = Layout::actions(&[&cancel, &submit], mtm);
        let stack = Layout::stack(
            &[
                &heading,
                &intro,
                &host_label,
                &input,
                &identity_label,
                &identity_choices,
                &status,
                &actions,
            ],
            NSUserInterfaceLayoutOrientation::Vertical,
            16.0,
            mtm,
        );
        content.addSubview(&stack);
        Layout::pin(&stack, &content, 24.0, 24.0, 20.0, 24.0);
        Layout::width(&stack, 472.0);
        let component = Arc::new(MainThreadBound::new(
            RefCell::new(Self {
                runtime: runtime.clone(),
                application,
                open_ssh: Arc::new(crate::system::open_ssh::MacOsOpenSshService::new(
                    home,
                    socket_path.into(),
                )),
                events,
                parent,
                sheet,
                input,
                identity_choices,
                identity_buttons: Vec::new(),
                status,
                submit,
                cancel,
                state: None,
                _static_targets: vec![submit_target, cancel_target],
                identity_targets: Vec::new(),
                _host_delegate: host_delegate,
                command_handler: None,
            }),
            mtm,
        ));
        let weak = Arc::downgrade(&component);
        receiver.spawn_on_main(&runtime, move |event, mtm| {
            let Some(component) = weak.upgrade() else {
                return;
            };
            component.get(mtm).borrow_mut().dispatch(event, mtm);
        });
        component
    }
    pub(crate) fn present(&mut self, props: SshConnectionTestProps, _mtm: MainThreadMarker) {
        self.sheet.setTitle(&NSString::from_str(
            &keyringd_ui_core::ssh_connection_test_window::profile_heading(
                props.profile.display_name(),
            ),
        ));
        self.input.setStringValue(&NSString::new());
        for view in self.identity_choices.arrangedSubviews() {
            self.identity_choices.removeArrangedSubview(&view);
            view.removeFromSuperview();
        }
        self.identity_buttons.clear();
        self.identity_targets.clear();
        let identities =
            ProfileSshIdentity::available_kinds_at(&props.profile, std::time::SystemTime::now());
        if identities.is_empty() {
            return;
        }
        let state = SshConnectionTestState::new(identities);
        self.command_handler = Some(SshConnectionTestCommandHandler::new(
            Arc::clone(&self.open_ssh),
            Arc::clone(&self.application),
            props.profile.id().clone(),
            self.runtime.clone(),
            self.events.clone(),
            self.parent.clone(),
            self.sheet.clone(),
        ));
        for (index, identity) in state.identities().iter().enumerate() {
            let (title, subtitle) = Self::identity_copy(identity);
            let target = SshConnectionTestActionTarget::new(
                _mtm,
                self.events.clone(),
                SshConnectionTestEvent::IdentitySelected(*identity),
            );
            let choice = unsafe {
                NSButton::radioButtonWithTitle_target_action(
                    &NSString::from_str(title),
                    Some(&target),
                    Some(sel!(performAction:)),
                    _mtm,
                )
            };
            choice.setAccessibilityIdentifier(Some(&NSString::from_str(match identity {
                ProfileSshIdentityKind::DeviceCertificate => {
                    "connection-test.identity.device-certificate"
                }
                ProfileSshIdentityKind::DeviceKey => "connection-test.identity.device-key",
                ProfileSshIdentityKind::Passkey => "connection-test.identity.passkey",
            })));
            if index == 0 {
                choice.setState(NSControlStateValueOn);
            }
            let description = Layout::label(subtitle, 11.0, false, _mtm);
            // AppKit groups radio buttons with the same action and superview.
            self.identity_choices.addArrangedSubview(&choice);
            self.identity_choices.addArrangedSubview(&description);
            self.identity_choices
                .setCustomSpacing_afterView(4.0, &choice);
            Layout::fill_width(&choice, &self.identity_choices);
            Layout::fill_width(&description, &self.identity_choices);
            self.identity_buttons.push(choice);
            self.identity_targets.push(target);
        }
        self.state = Some(ComponentRuntime::new(state));
        self.render();
        if self.sheet.sheetParent().is_none() {
            self.parent.beginSheet_completionHandler(&self.sheet, None);
        }
        self.sheet.makeFirstResponder(Some(&self.input));
    }
    fn dispatch(&mut self, event: SshConnectionTestEvent, mtm: MainThreadMarker) {
        let Some(state) = &self.state else { return };
        let command = state.dispatch(event);
        self.render();
        if let Some(command) = command
            && let Some(handler) = &mut self.command_handler
        {
            handler.handle(command, mtm);
        }
        if self.sheet.sheetParent().is_none() {
            self.state = None;
            self.command_handler = None;
        }
    }
    fn render(&self) {
        let Some(runtime) = &self.state else { return };
        let state = runtime.state();
        let testing = state.is_testing();
        self.input.setEnabled(state.can_edit());
        for choice in &self.identity_buttons {
            choice.setEnabled(state.can_edit());
        }
        self.submit.setEnabled(state.can_test());
        self.cancel.setEnabled(!testing);
        self.cancel.setTitle(&NSString::from_str(ui_text::CANCEL));
        self.status.setStringValue(&NSString::from_str(if testing {
            ui_text::SSH_CONNECTION_TEST_TESTING
        } else {
            state.error().unwrap_or_default()
        }));
        self.status.setHidden(!testing && state.error().is_none());
        let color = if state.error().is_some() {
            NSColor::systemRedColor()
        } else {
            NSColor::secondaryLabelColor()
        };
        self.status.setTextColor(Some(&color));
        if let Some(content) = self.sheet.contentView() {
            Layout::fit_window(&content);
        }
    }
    fn identity_copy(identity: &ProfileSshIdentityKind) -> (&'static str, &'static str) {
        (
            keyringd_ui_core::ssh_connection_test_window::identity_title(*identity),
            keyringd_ui_core::ssh_connection_test_window::identity_description(
                *identity,
                ui_text::SSH_CONNECTION_TEST_SECURE_ENCLAVE_KEY_DESCRIPTION,
            ),
        )
    }
}
