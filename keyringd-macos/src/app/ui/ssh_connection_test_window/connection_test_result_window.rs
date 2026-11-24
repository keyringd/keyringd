use crate::app::ui::{ControlActionTarget, Layout};
use block2::RcBlock;
use keyringd_core::service::SshConnectionTestReport;
use keyringd_ui_core::runtime::{UiEventSender, UiEventTarget};
use keyringd_ui_core::ssh_connection_test_window::SshConnectionTestPresentation;
use keyringd_ui_core::text as ui_text;
use objc2::{MainThreadMarker, MainThreadOnly, sel};
use objc2_app_kit::{
    NSAccessibility, NSBackingStoreType, NSButton, NSFont, NSModalResponse, NSTextField,
    NSUserInterfaceLayoutOrientation, NSWindow, NSWindowStyleMask,
};
use objc2_foundation::{NSPoint, NSRect, NSSize, NSString};

use super::SshConnectionTestActionTarget;
use super::SshConnectionTestEvent;

/// Shared reading width for every SSH result, including configuration diagnostics.
const CONTENT_WIDTH: f64 = 600.0;
const INSET: f64 = 24.0;

pub(super) struct ConnectionTestResultWindow;

impl ConnectionTestResultWindow {
    pub(super) fn present(
        parent: &NSWindow,
        target: &str,
        report: &SshConnectionTestReport,
        events: UiEventSender<SshConnectionTestEvent>,
        mtm: MainThreadMarker,
    ) -> crate::app::ui::toast::ToastOverlay {
        let presentation = SshConnectionTestPresentation::new(report);
        let body = presentation.body_with_host(target);
        let SshConnectionTestPresentation {
            heading,
            troubleshooting,
            ..
        } = presentation;
        let window = unsafe {
            NSWindow::initWithContentRect_styleMask_backing_defer(
                NSWindow::alloc(mtm),
                NSRect::new(
                    NSPoint::ZERO,
                    NSSize::new(CONTENT_WIDTH + 2.0 * INSET, 240.0),
                ),
                NSWindowStyleMask::Titled,
                NSBackingStoreType::Buffered,
                false,
            )
        };
        unsafe { window.setReleasedWhenClosed(false) };
        window.setTitle(&NSString::from_str(
            ui_text::SSH_CONNECTION_TEST_RESULT_WINDOW_TITLE,
        ));
        window.setAccessibilityIdentifier(Some(&NSString::from_str("connection-test.result")));
        let content = window
            .contentView()
            .expect("the initialized SSH result sheet should have a content view");

        let close_target = ControlActionTarget::new(
            mtm,
            UiEventTarget::new({
                let window = window.clone();
                move |()| {
                    if let Some(parent) = window.sheetParent() {
                        parent.endSheet(&window);
                    }
                }
            }),
            (),
        );
        let close = unsafe {
            NSButton::buttonWithTitle_target_action(
                &NSString::from_str(ui_text::CLOSE),
                Some(&close_target),
                Some(sel!(performAction:)),
                mtm,
            )
        };
        close.setKeyEquivalent(&NSString::from_str("\r"));
        close.setAccessibilityIdentifier(Some(&NSString::from_str("connection-test.result.close")));

        let copy_target = SshConnectionTestActionTarget::new(
            mtm,
            events.clone(),
            SshConnectionTestEvent::ResultCopyRequested,
        );
        let stack = Layout::stack(&[], NSUserInterfaceLayoutOrientation::Vertical, 12.0, mtm);
        let title = Self::label(&heading, "title", true, mtm);
        title.setFont(Some(&NSFont::boldSystemFontOfSize(20.0)));
        stack.addArrangedSubview(&title);
        stack.addArrangedSubview(&Self::label(&body, "body", false, mtm));
        if let Some((entry, server)) = &troubleshooting {
            stack.addArrangedSubview(&Self::label(
                ui_text::SSH_CONNECTION_TEST_AUTHORIZED_KEYS_HEADING,
                "authorized-keys-heading",
                true,
                mtm,
            ));
            stack.addArrangedSubview(&Self::label(
                ui_text::SSH_CONNECTION_TEST_AUTHORIZED_KEYS_EXPLANATION,
                "authorized-keys",
                false,
                mtm,
            ));
            let entry_label = Self::label(entry, "entry", false, mtm);
            entry_label.setSelectable(true);
            stack.addArrangedSubview(&entry_label);
            let copy = unsafe {
                NSButton::buttonWithTitle_target_action(
                    &NSString::from_str(ui_text::SSH_CONNECTION_TEST_COPY_ENTRY),
                    Some(&copy_target),
                    Some(sel!(performAction:)),
                    mtm,
                )
            };
            copy.setAccessibilityIdentifier(Some(&NSString::from_str(
                "connection-test.result.copy",
            )));
            stack.addArrangedSubview(&copy);
            stack.addArrangedSubview(&Self::label(
                ui_text::SSH_CONNECTION_TEST_SERVER_HEADING,
                "server-heading",
                true,
                mtm,
            ));
            stack.addArrangedSubview(&Self::label(server, "server", false, mtm));
        }
        let actions = Layout::actions(&[&close], mtm);
        stack.addArrangedSubview(&actions);
        Layout::fill_width(&actions, &stack);
        content.addSubview(&stack);
        Layout::pin(&stack, &content, INSET, INSET, INSET, INSET);
        Layout::width(&stack, CONTENT_WIDTH);
        Layout::fit_window(&content);
        let toast = crate::app::ui::toast::ToastOverlay::new(&content, mtm);

        let closing_window = window.clone();
        let completion = RcBlock::new(move |_: NSModalResponse| {
            // AppKit's button targets are weak. Keep them until the sheet ends.
            let _ = (&close_target, &copy_target);
            closing_window.orderOut(None);
            events.send(SshConnectionTestEvent::ResultClosed);
        });
        parent.beginSheet_completionHandler(&window, Some(&completion));
        toast
    }

    fn label(
        value: &str,
        identifier: &str,
        bold: bool,
        mtm: MainThreadMarker,
    ) -> objc2::rc::Retained<NSTextField> {
        let label = NSTextField::wrappingLabelWithString(&NSString::from_str(value), mtm);
        label.setAccessibilityIdentifier(Some(&NSString::from_str(&format!(
            "connection-test.result.{identifier}"
        ))));
        label.setTranslatesAutoresizingMaskIntoConstraints(false);
        label.setPreferredMaxLayoutWidth(CONTENT_WIDTH);
        label
            .widthAnchor()
            .constraintEqualToConstant(CONTENT_WIDTH)
            .setActive(true);
        let font = if bold {
            NSFont::boldSystemFontOfSize(13.0)
        } else {
            NSFont::systemFontOfSize(12.0)
        };
        label.setFont(Some(&font));
        label
    }
}
