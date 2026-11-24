use super::AboutWindowActionTarget;
use crate::app::ui::Layout;
use crate::app::ui::third_party_licenses_window::ThirdPartyLicensesWindow;
use keyringd_ui_core::about_window::LicenseDocument;
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{AnyThread, MainThreadMarker, MainThreadOnly, sel};
use objc2_app_kit::{
    NSAccessibility, NSBackingStoreType, NSButton, NSColor, NSFont, NSForegroundColorAttributeName,
    NSImage, NSImageView, NSLayoutAttribute, NSMenuItem, NSStackView, NSTextField,
    NSUserInterfaceLayoutOrientation, NSView, NSWindow, NSWindowStyleMask, NSWorkspace,
};
use objc2_foundation::{
    NSArray, NSAttributedString, NSDictionary, NSPoint, NSRect, NSSize, NSString, NSURL,
};

use keyringd_ui_core::text as ui_text;
use std::cell::RefCell;
use std::rc::Rc;

pub(crate) struct AboutWindow {
    window: RefCell<Option<Retained<NSWindow>>>,
    license: RefCell<Option<Rc<ThirdPartyLicensesWindow>>>,
    actions: Retained<AboutWindowActionTarget>,
    mtm: MainThreadMarker,
}

impl AboutWindow {
    pub(crate) fn new(mtm: MainThreadMarker) -> Rc<Self> {
        Rc::new_cyclic(|owner| Self {
            window: RefCell::new(None),
            license: RefCell::new(None),
            actions: AboutWindowActionTarget::new(mtm, owner.clone()),
            mtm,
        })
    }

    pub(in crate::app::ui) fn bind_menu_item(&self, item: &NSMenuItem) {
        unsafe {
            item.setTarget(Some(&*self.actions));
        }
    }

    pub(super) fn close(&self) {
        if let Some(window) = self.window.borrow().as_ref() {
            window.close();
        }
    }

    pub(super) fn show_license(&self) {
        self.show_license_document(LicenseDocument::Application);
    }

    pub(super) fn show_third_party_licenses(&self) {
        self.show_license_document(LicenseDocument::ThirdParty);
    }

    fn show_license_document(&self, document: LicenseDocument) {
        if let Some(owner) = self.window.borrow().as_ref()
            && let Some(window) = ThirdPartyLicensesWindow::present(owner, document, self.mtm)
        {
            self.license.replace(Some(window));
        }
    }

    pub(crate) fn show(&self) {
        if let Some(window) = self.window.borrow().as_ref() {
            window.makeKeyAndOrderFront(None);
            return;
        }
        let window = self.window(ui_text::ABOUT_TITLE, 480.0, 320.0);
        let content = window
            .contentView()
            .expect("the initialized About window should have the content view supplied by AppKit");
        let icon = NSImageView::new(self.mtm);
        icon.setImage(
            NSImage::imageNamed(unsafe { objc2_app_kit::NSImageNameApplicationIcon }).as_deref(),
        );
        Layout::width(&icon, 64.0);
        Layout::height(&icon, 64.0);
        let name = self.label("Keyringd", "about.name", 24.0);
        name.setFont(Some(&NSFont::boldSystemFontOfSize(24.0)));
        let version = self.label(
            &keyringd_ui_core::about_window::version(env!("CARGO_PKG_VERSION")),
            "about.version",
            13.0,
        );
        let developers = self.label(ui_text::ABOUT_MAINTAINERS, "about.developers", 13.0);
        let license = self.label(ui_text::ABOUT_LICENSE_NAME, "about.license-name", 13.0);
        let links = self.stack(
            &[
                &self.link(ui_text::ABOUT_WEBSITE, "about.website", sel!(openWebsite:)),
                &self.link(
                    ui_text::ABOUT_REPORT_ISSUE,
                    "about.issues",
                    sel!(reportIssue:),
                ),
            ],
            NSUserInterfaceLayoutOrientation::Horizontal,
        );
        let documents = self.stack(
            &[
                &self.button(ui_text::ABOUT_LICENSE, "about.license", sel!(showLicense:)),
                &self.button(
                    ui_text::ABOUT_THIRD_PARTY_LICENSES,
                    "about.third-party",
                    sel!(showThirdPartyLicenses:),
                ),
            ],
            NSUserInterfaceLayoutOrientation::Horizontal,
        );
        let close = self.button(ui_text::CLOSE, "about.close", sel!(closeAbout:));
        close.setKeyEquivalent(&NSString::from_str("\u{1b}"));
        let body = self.stack(
            &[&icon, &name, &version, &developers, &license, &links],
            NSUserInterfaceLayoutOrientation::Vertical,
        );
        body.setSpacing(8.0);
        content.addSubview(&body);
        content.addSubview(&documents);
        content.addSubview(&close);
        Layout::prepare(&body);
        Layout::prepare(&documents);
        Layout::prepare(&close);
        body.topAnchor()
            .constraintEqualToAnchor_constant(&content.topAnchor(), 24.0)
            .setActive(true);
        body.leadingAnchor()
            .constraintEqualToAnchor_constant(&content.leadingAnchor(), 24.0)
            .setActive(true);
        body.trailingAnchor()
            .constraintEqualToAnchor_constant(&content.trailingAnchor(), -24.0)
            .setActive(true);
        body.bottomAnchor()
            .constraintLessThanOrEqualToAnchor_constant(&documents.topAnchor(), -20.0)
            .setActive(true);
        documents
            .leadingAnchor()
            .constraintEqualToAnchor_constant(&content.leadingAnchor(), 24.0)
            .setActive(true);
        documents
            .bottomAnchor()
            .constraintEqualToAnchor_constant(&content.bottomAnchor(), -20.0)
            .setActive(true);
        documents
            .trailingAnchor()
            .constraintLessThanOrEqualToAnchor_constant(&close.leadingAnchor(), -12.0)
            .setActive(true);
        close
            .trailingAnchor()
            .constraintEqualToAnchor_constant(&content.trailingAnchor(), -24.0)
            .setActive(true);
        close
            .centerYAnchor()
            .constraintEqualToAnchor(&documents.centerYAnchor())
            .setActive(true);
        window.center();
        window.makeKeyAndOrderFront(None);
        self.window.replace(Some(window));
    }
    fn window(&self, title: &str, width: f64, height: f64) -> Retained<NSWindow> {
        let window = unsafe {
            NSWindow::initWithContentRect_styleMask_backing_defer(
                NSWindow::alloc(self.mtm),
                NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(width, height)),
                NSWindowStyleMask::Titled | NSWindowStyleMask::Closable,
                NSBackingStoreType::Buffered,
                false,
            )
        };
        unsafe { window.setReleasedWhenClosed(false) };
        window.setTitle(&NSString::from_str(title));
        window
    }
    fn label(&self, value: &str, id: &str, size: f64) -> Retained<NSTextField> {
        let label = NSTextField::wrappingLabelWithString(&NSString::from_str(value), self.mtm);
        label.setFont(Some(&NSFont::systemFontOfSize(size)));
        label.setAccessibilityIdentifier(Some(&NSString::from_str(id)));
        label
    }
    fn button(&self, title: &str, id: &str, action: objc2::runtime::Sel) -> Retained<NSButton> {
        let button = unsafe {
            NSButton::buttonWithTitle_target_action(
                &NSString::from_str(title),
                Some(&*self.actions),
                Some(action),
                self.mtm,
            )
        };
        button.setAccessibilityIdentifier(Some(&NSString::from_str(id)));
        button
    }
    fn link(&self, title: &str, id: &str, action: objc2::runtime::Sel) -> Retained<NSButton> {
        let button = self.button(title, id, action);
        button.setBordered(false);
        let color = NSColor::linkColor();
        let attributes = NSDictionary::<NSString, AnyObject>::from_slices(
            &[unsafe { NSForegroundColorAttributeName }],
            &[&*color],
        );
        let title = unsafe {
            NSAttributedString::initWithString_attributes(
                NSAttributedString::alloc(),
                &NSString::from_str(title),
                Some(&attributes),
            )
        };
        button.setAttributedTitle(&title);
        button
    }
    fn stack(
        &self,
        views: &[&NSView],
        orientation: NSUserInterfaceLayoutOrientation,
    ) -> Retained<NSStackView> {
        let stack = NSStackView::stackViewWithViews(&NSArray::from_slice(views), self.mtm);
        stack.setOrientation(orientation);
        stack.setAlignment(
            if orientation == NSUserInterfaceLayoutOrientation::Vertical {
                NSLayoutAttribute::CenterX
            } else {
                NSLayoutAttribute::CenterY
            },
        );
        stack.setSpacing(12.0);
        stack
    }
    pub(super) fn open_url(&self, value: &str) {
        if let Some(url) = NSURL::URLWithString(&NSString::from_str(value)) {
            NSWorkspace::sharedWorkspace().openURL(&url);
        }
    }
}
