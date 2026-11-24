use keyringd_ui_core::text as ui_text;
use std::cell::RefCell;
use std::sync::Arc;

use super::ToastActionTarget;
use dispatch2::{DispatchQueue, DispatchTime, MainThreadBound};
use objc2::rc::Retained;
use objc2::{MainThreadMarker, MainThreadOnly, sel};
use objc2_app_kit::{
    NSAccessibility, NSBox, NSBoxType, NSButton, NSColor, NSFont, NSTextField, NSTitlePosition,
    NSView,
};
use objc2_foundation::{NSRect, NSSize, NSString};

use crate::app::ui::Layout;
use keyringd_ui_core::toast::{ToastMessage, ToastState};

struct ToastInner {
    pill: Retained<NSBox>,
    label: Retained<NSTextField>,
    dismiss_button: Retained<NSButton>,
    dismiss_target: RefCell<Option<Retained<ToastActionTarget>>>,
    state: RefCell<ToastState>,
}

pub(crate) struct ToastOverlay {
    inner: Arc<MainThreadBound<ToastInner>>,
}

impl ToastOverlay {
    pub(crate) fn new(parent: &NSView, mtm: MainThreadMarker) -> Self {
        let pill = NSBox::initWithFrame(NSBox::alloc(mtm), NSRect::ZERO);
        pill.setBoxType(NSBoxType::Custom);
        pill.setTitlePosition(NSTitlePosition::NoTitle);
        pill.setBorderWidth(0.0);
        pill.setCornerRadius(8.0);
        pill.setContentViewMargins(NSSize::new(0.0, 0.0));
        pill.setHidden(true);
        Layout::prepare(&pill);

        let label = NSTextField::wrappingLabelWithString(&NSString::from_str(""), mtm);
        label.setAccessibilityIdentifier(Some(&NSString::from_str("feedback.toast-message")));
        label.setFont(Some(&NSFont::systemFontOfSize(13.0)));
        label.setTextColor(Some(&NSColor::labelColor()));
        label.setAlignment(objc2_app_kit::NSTextAlignment::Center);
        pill.addSubview(&label);
        Layout::prepare(&label);
        Layout::pin(&label, &pill, 8.0, 16.0, 8.0, 48.0);
        let dismiss_button = unsafe {
            NSButton::buttonWithTitle_target_action(
                &NSString::from_str("×"),
                None,
                Some(sel!(dismissToast:)),
                mtm,
            )
        };
        dismiss_button.setAccessibilityLabel(Some(&NSString::from_str(ui_text::TOAST_DISMISS)));
        pill.addSubview(&dismiss_button);
        Layout::prepare(&dismiss_button);
        dismiss_button
            .trailingAnchor()
            .constraintEqualToAnchor_constant(&pill.trailingAnchor(), -8.0)
            .setActive(true);
        dismiss_button
            .centerYAnchor()
            .constraintEqualToAnchor(&pill.centerYAnchor())
            .setActive(true);

        parent.addSubview(&pill);
        pill.centerXAnchor()
            .constraintEqualToAnchor(&parent.centerXAnchor())
            .setActive(true);
        pill.bottomAnchor()
            .constraintEqualToAnchor_constant(&parent.bottomAnchor(), -16.0)
            .setActive(true);

        let inner = Arc::new(MainThreadBound::new(
            ToastInner {
                pill,
                label,
                dismiss_button,
                dismiss_target: RefCell::new(None),
                state: RefCell::new(ToastState::default()),
            },
            mtm,
        ));
        Self { inner }
    }

    pub(crate) fn show(&self, message: ToastMessage, mtm: MainThreadMarker) {
        let inner = self.inner.get(mtm);
        let (revision, text, is_error, timeout) = {
            let mut state = inner.state.borrow_mut();
            let revision = state.show(message);
            let message = state.message().expect("a toast was just shown");
            (
                revision,
                message.text().to_owned(),
                message.is_error(),
                state.auto_dismiss_after(),
            )
        };
        let text_color = if is_error {
            NSColor::systemRedColor()
        } else {
            NSColor::labelColor()
        };

        inner.label.setStringValue(&NSString::from_str(&text));
        inner.label.setTextColor(Some(&text_color));
        inner.pill.setFillColor(&NSColor::controlBackgroundColor());
        inner.pill.setBorderColor(&NSColor::separatorColor());
        inner.pill.setBorderWidth(1.0);
        inner.pill.setHidden(false);
        let weak = Arc::downgrade(&self.inner);
        let target = ToastActionTarget::new(mtm, move || {
            let Some(inner) = weak.upgrade() else { return };
            let inner = inner.get(mtm);
            if inner.state.borrow_mut().dismiss(revision) {
                inner.pill.setHidden(true);
            }
        });
        unsafe {
            inner.dismiss_button.setTarget(Some(&target));
        }
        *inner.dismiss_target.borrow_mut() = Some(target);

        let Some(timeout) = timeout else { return };
        let weak = Arc::downgrade(&self.inner);
        if let Ok(when) = DispatchTime::try_from(timeout) {
            let _ = DispatchQueue::main().after(when, move || {
                let mtm = MainThreadMarker::new()
                    .expect("the callback should run on the AppKit thread because it was dispatched to the GCD main queue");
                let Some(inner) = weak.upgrade() else {
                    return;
                };
                let inner = inner.get(mtm);
                if inner.state.borrow_mut().expire(revision) {
                    inner.pill.setHidden(true);
                }
            });
        }
    }
}
