use super::SshClientConfigurationActionTarget;
use super::SshClientConfigurationCommandHandler;
use super::SshClientConfigurationEvent;
use super::SshClientConfigurationState;
use crate::app::ui::ComponentRuntime;
use keyringd_ui_core::runtime::{UiEventReceiver, UiEventSender};
use objc2::rc::Retained;
use objc2::{MainThreadMarker, MainThreadOnly, sel};
use objc2_app_kit::{
    NSAccessibility, NSBackingStoreType, NSBox, NSBoxType, NSButton, NSColor, NSFont, NSTextField,
    NSTitlePosition, NSView, NSWindow, NSWindowStyleMask,
};
use objc2_foundation::{NSPoint, NSRect, NSSize, NSString};

use keyringd_ui_core::text as ui_text;
use std::sync::Arc;
pub(crate) struct SshClientConfigurationWindow {
    runtime: ComponentRuntime<SshClientConfigurationState>,
    command_handler: SshClientConfigurationCommandHandler,
    window: Retained<NSWindow>,
    status: Retained<NSTextField>,
    configure: Retained<NSButton>,
    _configure_target: Retained<SshClientConfigurationActionTarget>,
}
impl SshClientConfigurationWindow {
    pub(crate) fn new(
        selection: Arc<crate::app::SshAgentSelectionService>,
        task_runtime: tokio::runtime::Handle,
        mtm: MainThreadMarker,
    ) -> (Self, UiEventReceiver<SshClientConfigurationEvent>) {
        let (events, receiver) = UiEventSender::channel();
        let window = unsafe {
            NSWindow::initWithContentRect_styleMask_backing_defer(
                NSWindow::alloc(mtm),
                NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(620.0, 300.0)),
                NSWindowStyleMask::Titled
                    | NSWindowStyleMask::Closable
                    | NSWindowStyleMask::Miniaturizable,
                NSBackingStoreType::Buffered,
                false,
            )
        };
        window.setTitle(&NSString::from_str(
            ui_text::SSH_CLIENT_CONFIGURATION_WINDOW_TITLE,
        ));
        unsafe { window.setReleasedWhenClosed(false) };
        window.center();
        let content = window
            .contentView()
            .expect("the initialized SSH client configuration window should have the content view supplied by AppKit");
        Self::label(
            &content,
            ui_text::SSH_CLIENT_CONFIGURATION_HEADING,
            32.0,
            234.0,
            556.0,
            34.0,
            24.0,
            true,
            mtm,
        );
        Self::label(
            &content,
            ui_text::SSH_CLIENT_CONFIGURATION_DESCRIPTION,
            32.0,
            184.0,
            556.0,
            40.0,
            13.0,
            false,
            mtm,
        );
        let integration = NSBox::initWithFrame(
            NSBox::alloc(mtm),
            NSRect::new(NSPoint::new(32.0, 76.0), NSSize::new(556.0, 88.0)),
        );
        integration.setBoxType(NSBoxType::Primary);
        integration.setTitlePosition(NSTitlePosition::NoTitle);
        integration.setContentViewMargins(NSSize::new(0.0, 0.0));
        content.addSubview(&integration);
        let integration = integration
            .contentView()
            .expect("the initialized OpenSSH integration box should have the content view supplied by AppKit");
        Self::label(
            &integration,
            "SSH_AUTH_SOCK",
            16.0,
            48.0,
            316.0,
            24.0,
            15.0,
            true,
            mtm,
        );
        Self::label(
            &integration,
            ui_text::SSH_CLIENT_CONFIGURATION_AGENT_DESCRIPTION,
            16.0,
            20.0,
            316.0,
            22.0,
            12.0,
            false,
            mtm,
        );
        let status = Self::label(&content, "", 32.0, 32.0, 380.0, 32.0, 12.0, false, mtm);
        status.setAccessibilityIdentifier(Some(&NSString::from_str("getting-started.status")));
        let configure_target = SshClientConfigurationActionTarget::new(mtm, events.clone());
        let configure = unsafe {
            NSButton::buttonWithTitle_target_action(
                &NSString::from_str(ui_text::SSH_CLIENT_CONFIGURATION_CONFIGURE),
                Some(&configure_target),
                Some(sel!(performAction:)),
                mtm,
            )
        };
        configure
            .setAccessibilityIdentifier(Some(&NSString::from_str("getting-started.configure")));
        configure.setFrame(NSRect::new(
            NSPoint::new(356.0, 28.0),
            NSSize::new(184.0, 34.0),
        ));
        integration.addSubview(&configure);
        let command_handler = SshClientConfigurationCommandHandler::new(
            selection,
            task_runtime,
            events,
            window.clone(),
        );
        (
            Self {
                runtime: ComponentRuntime::new(SshClientConfigurationState::new(
                    command_handler.has_managed_agent_configuration(),
                )),
                command_handler,
                window,
                status,
                configure,
                _configure_target: configure_target,
            },
            receiver,
        )
    }
    pub(crate) fn dispatch(&self, event: SshClientConfigurationEvent, _mtm: MainThreadMarker) {
        if matches!(
            &event,
            SshClientConfigurationEvent::OpenRequested
                | SshClientConfigurationEvent::ConfigureRequested
        ) && !self.runtime.state().busy()
        {
            let _ = self
                .runtime
                .dispatch(SshClientConfigurationEvent::ConfigurationChecked(
                    self.command_handler.has_managed_agent_configuration(),
                ));
        }
        let command = self.runtime.dispatch(event);
        self.render();
        if let Some(command) = command {
            self.command_handler
                .handle(command, self.runtime.state().configured(), _mtm);
        }
    }
    fn render(&self) {
        let state = self.runtime.state();
        self.configure.setEnabled(!state.busy());
        self.configure
            .setTitle(&NSString::from_str(state.button_label()));
        let (text, error) = state.status();
        self.status.setStringValue(&NSString::from_str(text));
        let color = if error {
            NSColor::systemRedColor()
        } else {
            NSColor::secondaryLabelColor()
        };
        self.status.setTextColor(Some(&color));
        if state.is_open() {
            self.window.makeKeyAndOrderFront(None);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn label(
        content: &NSView,
        text: &str,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        size: f64,
        bold: bool,
        mtm: MainThreadMarker,
    ) -> Retained<NSTextField> {
        let label = NSTextField::wrappingLabelWithString(&NSString::from_str(text), mtm);
        label.setFrame(NSRect::new(NSPoint::new(x, y), NSSize::new(width, height)));
        let font = if bold {
            NSFont::boldSystemFontOfSize(size)
        } else {
            NSFont::systemFontOfSize(size)
        };
        label.setFont(Some(&font));
        if !bold {
            label.setTextColor(Some(&NSColor::secondaryLabelColor()));
        }
        content.addSubview(&label);
        label
    }
}
