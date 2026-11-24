use keyringd_fido_client::HybridTransportUnavailableReason;
use keyringd_ui_core::qr_prompt_window::HybridQrPresentation;
use objc2::rc::Retained;
use objc2::{MainThreadMarker, MainThreadOnly, sel};
use objc2_app_kit::{
    NSAccessibility, NSBackingStoreType, NSButton, NSColor, NSFont, NSTextField, NSView, NSWindow,
    NSWindowStyleMask,
};
use objc2_foundation::{NSPoint, NSRect, NSSize, NSString};

use keyringd_ui_core::runtime::UiEventSender;
use keyringd_ui_core::text as ui_text;
use tokio::sync::oneshot;

use super::QrPromptActionTarget;
use crate::app::ui::AppShell;
use crate::app::ui::Layout;
use crate::app::ui::PASSKEY_DIALOG_WIDTH;
use crate::app::ui::qr_prompt_window::HybridQrView;

pub(crate) struct QrPromptWindow {
    mtm: MainThreadMarker,
    shell: Retained<AppShell>,
    prompt: Option<HybridQrView>,
    target: Option<Retained<QrPromptActionTarget>>,
}

impl QrPromptWindow {
    pub(crate) fn new(mtm: MainThreadMarker, shell: Retained<AppShell>) -> Self {
        Self {
            mtm,
            shell,
            prompt: None,
            target: None,
        }
    }

    pub(crate) fn present(
        &mut self,
        presentation: HybridQrPresentation,
        prompt_id: keyringd_fido_client::InteractionId,
        presented: oneshot::Sender<()>,
        events: UiEventSender<keyringd_ui_core::key_operation::KeyOperationEvent>,
    ) {
        self.dismiss();
        let window = unsafe {
            NSWindow::initWithContentRect_styleMask_backing_defer(
                NSWindow::alloc(self.mtm),
                NSRect::new(
                    NSPoint::new(0., 0.),
                    NSSize::new(PASSKEY_DIALOG_WIDTH, 536.),
                ),
                NSWindowStyleMask::Titled,
                NSBackingStoreType::Buffered,
                false,
            )
        };
        let parent = self.shell.anchor();
        let target = QrPromptActionTarget::new(
            self.mtm,
            events,
            prompt_id,
            window.clone(),
            self.shell.clone(),
            parent.clone(),
        );
        window.setAccessibilityIdentifier(Some(&NSString::from_str("key-operation.prompt")));
        window.setTitle(&NSString::from_str(presentation.title()));
        unsafe { window.setReleasedWhenClosed(false) };
        let content = window.contentView().expect(
            "the initialized Hybrid prompt window should have the content view supplied by AppKit",
        );
        let heading = Self::label(&content, presentation.title(), 20.0, true, self.mtm);
        heading
            .topAnchor()
            .constraintEqualToAnchor_constant(&content.topAnchor(), 24.0)
            .setActive(true);
        let detail = Self::label(&content, &presentation.explanation(), 13.0, false, self.mtm);
        detail
            .topAnchor()
            .constraintEqualToAnchor_constant(&heading.bottomAnchor(), 8.0)
            .setActive(true);
        let qr_view = HybridQrView::new(&presentation, self.mtm);
        content.addSubview(qr_view.view());
        Layout::prepare(qr_view.view());
        qr_view
            .view()
            .topAnchor()
            .constraintEqualToAnchor_constant(&detail.bottomAnchor(), 20.0)
            .setActive(true);
        qr_view
            .view()
            .leadingAnchor()
            .constraintEqualToAnchor_constant(&content.leadingAnchor(), 24.0)
            .setActive(true);
        qr_view
            .view()
            .trailingAnchor()
            .constraintEqualToAnchor_constant(&content.trailingAnchor(), -24.0)
            .setActive(true);
        let cancel = unsafe {
            NSButton::buttonWithTitle_target_action(
                &NSString::from_str(ui_text::CANCEL),
                Some(&target),
                Some(sel!(cancelOperation:)),
                self.mtm,
            )
        };
        cancel.setAccessibilityIdentifier(Some(&NSString::from_str("key-operation.cancel")));
        content.addSubview(&cancel);
        Layout::prepare(&cancel);
        cancel
            .trailingAnchor()
            .constraintEqualToAnchor_constant(&content.trailingAnchor(), -24.0)
            .setActive(true);
        cancel
            .bottomAnchor()
            .constraintEqualToAnchor_constant(&content.bottomAnchor(), -20.0)
            .setActive(true);
        cancel
            .widthAnchor()
            .constraintGreaterThanOrEqualToConstant(80.0)
            .setActive(true);
        qr_view
            .view()
            .bottomAnchor()
            .constraintLessThanOrEqualToAnchor_constant(&cancel.topAnchor(), -20.0)
            .setActive(true);
        cancel.setKeyEquivalent(&NSString::from_str("\u{1b}"));
        if presentation.security_key_available {
            let use_security_key = unsafe {
                NSButton::buttonWithTitle_target_action(
                    &NSString::from_str(ui_text::QR_PROMPT_USE_SECURITY_KEY),
                    Some(&target),
                    Some(sel!(useSecurityKey:)),
                    self.mtm,
                )
            };
            use_security_key.setAccessibilityIdentifier(Some(&NSString::from_str(
                "key-operation.use-security-key",
            )));
            content.addSubview(&use_security_key);
            Layout::prepare(&use_security_key);
            use_security_key
                .leadingAnchor()
                .constraintEqualToAnchor_constant(&content.leadingAnchor(), 24.0)
                .setActive(true);
            use_security_key
                .topAnchor()
                .constraintEqualToAnchor_constant(&qr_view.view().bottomAnchor(), 12.0)
                .setActive(true);
            use_security_key
                .bottomAnchor()
                .constraintLessThanOrEqualToAnchor_constant(&cancel.topAnchor(), -20.0)
                .setActive(true);
        }
        match parent {
            Some(parent) => {
                parent.beginSheet_completionHandler(&window, None);
            }
            None => {
                window.center();
                self.shell.request_attention();
                window.makeKeyAndOrderFront(None);
            }
        };
        self.target = Some(target);
        self.prompt = Some(qr_view);
        let _ = presented.send(());
    }

    pub(crate) fn dismiss(&mut self) {
        if let Some(target) = self.target.take() {
            target.dismiss();
        }
        self.prompt = None;
    }

    pub(crate) fn usb_selected(&self) {
        if let Some(target) = &self.target {
            target.usb_selected();
        }
    }

    pub(crate) fn show_unavailable(&self, reason: HybridTransportUnavailableReason) {
        let Some(prompt) = &self.prompt else { return };
        prompt.show_unavailable(reason);
    }

    fn label(
        content: &NSView,
        text: &str,
        size: f64,
        bold: bool,
        mtm: MainThreadMarker,
    ) -> Retained<NSTextField> {
        let label = NSTextField::wrappingLabelWithString(&NSString::from_str(text), mtm);
        content.addSubview(&label);
        Layout::prepare(&label);
        label
            .leadingAnchor()
            .constraintEqualToAnchor_constant(&content.leadingAnchor(), 24.0)
            .setActive(true);
        label
            .trailingAnchor()
            .constraintEqualToAnchor_constant(&content.trailingAnchor(), -24.0)
            .setActive(true);
        label.setAlignment(objc2_app_kit::NSTextAlignment::Center);
        let font = if bold {
            NSFont::boldSystemFontOfSize(size)
        } else {
            NSFont::systemFontOfSize(size)
        };
        label.setFont(Some(&font));
        if !bold {
            label.setTextColor(Some(&NSColor::secondaryLabelColor()));
        }
        label
    }
}
