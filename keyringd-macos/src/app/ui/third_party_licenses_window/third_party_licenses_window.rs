use super::ThirdPartyLicensesWindowActionTarget;
use crate::app::ui::Layout;
use crate::system::application_resources::{APPLICATION_LICENSE, THIRD_PARTY_LICENSES};
use keyringd_ui_core::about_window::LicenseDocument;
use keyringd_ui_core::text as ui_text;
use objc2::rc::Retained;
use objc2::{MainThreadMarker, MainThreadOnly, sel};
use objc2_app_kit::{
    NSAccessibility, NSBackingStoreType, NSButton, NSColor, NSFont, NSScrollView, NSTextView,
    NSWindow, NSWindowStyleMask,
};
use objc2_foundation::{NSPoint, NSRect, NSSize, NSString};
use std::rc::Rc;

pub(crate) struct ThirdPartyLicensesWindow {
    window: Retained<NSWindow>,
    _actions: Retained<ThirdPartyLicensesWindowActionTarget>,
}
impl ThirdPartyLicensesWindow {
    pub(crate) fn present(
        owner: &NSWindow,
        document: LicenseDocument,
        mtm: MainThreadMarker,
    ) -> Option<Rc<Self>> {
        if owner.attachedSheet().is_some() {
            return None;
        }
        let window = unsafe {
            NSWindow::initWithContentRect_styleMask_backing_defer(
                NSWindow::alloc(mtm),
                NSRect::new(NSPoint::new(0., 0.), NSSize::new(680., 520.)),
                NSWindowStyleMask::Titled,
                NSBackingStoreType::Buffered,
                false,
            )
        };
        unsafe { window.setReleasedWhenClosed(false) };
        window.setTitle(&NSString::from_str(document.title()));
        let component = Rc::new_cyclic(|owner| Self {
            window: window.clone(),
            _actions: ThirdPartyLicensesWindowActionTarget::new(mtm, owner.clone()),
        });
        let content = window
            .contentView()
            .expect("an initialized license window has content");
        let scroll = NSScrollView::new(mtm);
        scroll.setHasVerticalScroller(true);
        scroll.setAutohidesScrollers(true);
        let view = NSTextView::initWithFrame(
            NSTextView::alloc(mtm),
            NSRect::new(NSPoint::new(0., 0.), NSSize::new(620., 440.)),
        );
        view.setString(&NSString::from_str(match document {
            LicenseDocument::Application => APPLICATION_LICENSE,
            LicenseDocument::ThirdParty => THIRD_PARTY_LICENSES,
        }));
        view.setEditable(false);
        view.setSelectable(true);
        view.setRichText(false);
        view.setFont(Some(&NSFont::systemFontOfSize(12.)));
        view.setTextColor(Some(&NSColor::labelColor()));
        view.setBackgroundColor(&NSColor::textBackgroundColor());
        view.setVerticallyResizable(true);
        view.setHorizontallyResizable(false);
        view.setAccessibilityIdentifier(Some(&NSString::from_str("about.license-text")));
        view.sizeToFit();
        scroll.setDocumentView(Some(&view));
        let close = unsafe {
            NSButton::buttonWithTitle_target_action(
                &NSString::from_str(ui_text::CLOSE),
                Some(&*component._actions),
                Some(sel!(closeDocument:)),
                mtm,
            )
        };
        close.setAccessibilityIdentifier(Some(&NSString::from_str("about.license-close")));
        close.setKeyEquivalent(&NSString::from_str("\r"));
        content.addSubview(&scroll);
        content.addSubview(&close);
        Layout::prepare(&scroll);
        Layout::prepare(&close);
        Layout::pin(&scroll, &content, 20., 20., 64., 20.);
        close
            .trailingAnchor()
            .constraintEqualToAnchor_constant(&content.trailingAnchor(), -20.)
            .setActive(true);
        close
            .bottomAnchor()
            .constraintEqualToAnchor_constant(&content.bottomAnchor(), -20.)
            .setActive(true);
        owner.beginSheet_completionHandler(&window, None);
        Some(component)
    }
    pub(super) fn close(&self) {
        if let Some(parent) = self.window.sheetParent() {
            parent.endSheet(&self.window);
        }
        self.window.orderOut(None);
    }
}
