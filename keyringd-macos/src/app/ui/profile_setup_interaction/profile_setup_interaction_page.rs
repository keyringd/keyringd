use keyringd_ui_core::profile_setup_window::{
    ProfileSetupInteractionPresentation, ProfileSetupInteractionScreen,
};
use keyringd_ui_core::runtime::UiEventTarget;
use keyringd_ui_core::text as ui_text;
use objc2::rc::Retained;
use objc2::{MainThreadMarker, sel};
use objc2_app_kit::{
    NSAccessibility, NSButton, NSColor, NSFont, NSLayoutAttribute, NSProgressIndicator,
    NSProgressIndicatorStyle, NSStackView, NSTextField, NSUserInterfaceLayoutOrientation, NSView,
};
use objc2_foundation::{NSArray, NSString};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

use crate::app::ui::Layout;
use crate::app::ui::pin_prompt_window::PinPromptView;
use crate::app::ui::profile_setup_wizard::{ProfileSetupActionTarget, ProfileSetupEvent};
use crate::app::ui::qr_prompt_window::HybridQrView;
use keyringd_ui_core::qr_prompt_window::HybridQrPresentation;

/// Every prompt uses a vertical layout and the shared QR component.
pub(crate) struct ProfileSetupInteractionPage {
    root: Retained<NSView>,
    pin_request: Rc<Cell<Option<keyringd_fido_client::InteractionId>>>,
    qr_page: Retained<NSStackView>,
    qr_content: Retained<NSView>,
    qr_value: RefCell<Option<String>>,
    qr_view: RefCell<Option<HybridQrView>>,
    qr_instruction: Retained<NSTextField>,
    key_page: Retained<NSStackView>,
    title: Retained<NSTextField>,
    explanation: Retained<NSTextField>,
    pin: Rc<PinPromptView>,
    spinner: Retained<NSProgressIndicator>,
    pin_submit: Retained<NSButton>,
    use_security_key: Retained<NSButton>,
    use_security_key_target: Retained<ProfileSetupActionTarget>,
    mtm: MainThreadMarker,
}

impl ProfileSetupInteractionPage {
    pub(crate) fn new(
        parent: &NSView,
        events: UiEventTarget<ProfileSetupEvent>,
        mtm: MainThreadMarker,
    ) -> Self {
        let root = NSView::new(mtm);
        parent.addSubview(&root);
        Layout::pin(&root, parent, 0.0, 0.0, 0.0, 0.0);

        let pin_submit = unsafe {
            NSButton::buttonWithTitle_target_action(
                &NSString::from_str(ui_text::CONTINUE),
                None,
                None,
                mtm,
            )
        };
        pin_submit
            .setAccessibilityIdentifier(Some(&NSString::from_str("profile-setup.pin-submit")));
        Layout::height(&pin_submit, 32.0);
        pin_submit
            .widthAnchor()
            .constraintGreaterThanOrEqualToConstant(80.0)
            .setActive(true);
        let use_security_key_target = ProfileSetupActionTarget::new(mtm, events.clone(), None);
        let use_security_key = unsafe {
            NSButton::buttonWithTitle_target_action(
                &NSString::from_str(ui_text::QR_PROMPT_USE_SECURITY_KEY),
                Some(&use_security_key_target),
                Some(sel!(performAction:)),
                mtm,
            )
        };
        use_security_key.setAccessibilityIdentifier(Some(&NSString::from_str(
            "profile-setup.use-security-key",
        )));
        Layout::height(&use_security_key, 32.0);
        let qr_content = NSView::new(mtm);
        let qr_instruction = Self::label(13.0, false, mtm);
        qr_instruction.setAlignment(objc2_app_kit::NSTextAlignment::Center);
        let qr_page = NSStackView::stackViewWithViews(
            &NSArray::<NSView>::from_slice(&[&*qr_instruction, &*qr_content, &*use_security_key]),
            mtm,
        );
        qr_page.setOrientation(NSUserInterfaceLayoutOrientation::Vertical);
        qr_page.setAlignment(NSLayoutAttribute::Leading);
        qr_page.setSpacing(12.0);
        root.addSubview(&qr_page);
        Layout::prepare(&qr_page);
        qr_page
            .topAnchor()
            .constraintEqualToAnchor(&root.topAnchor())
            .setActive(true);
        qr_page
            .leadingAnchor()
            .constraintEqualToAnchor(&root.leadingAnchor())
            .setActive(true);
        qr_page
            .trailingAnchor()
            .constraintEqualToAnchor(&root.trailingAnchor())
            .setActive(true);
        Layout::fill_width(&qr_instruction, &qr_page);
        Layout::fill_width(&qr_content, &qr_page);
        let title = Self::label(17.0, true, mtm);
        title.setAccessibilityIdentifier(Some(&NSString::from_str("profile-setup.prompt-title")));
        let explanation = Self::label(13.0, false, mtm);
        explanation.setAccessibilityIdentifier(Some(&NSString::from_str(
            "profile-setup.prompt-explanation",
        )));
        let pin_request = Rc::new(Cell::new(None));
        let response_request = pin_request.clone();
        let pin_events = events.clone();
        let pin = PinPromptView::new(
            &pin_submit,
            None,
            "profile-setup",
            UiEventTarget::new(move |command| {
                if let Some(request_id) = response_request.get() {
                    pin_events.send(ProfileSetupEvent::PinCompleted {
                        request_id,
                        command,
                    });
                }
            }),
            mtm,
        );
        let spinner = NSProgressIndicator::new(mtm);
        spinner.setStyle(NSProgressIndicatorStyle::Spinning);
        spinner.setIndeterminate(true);
        spinner.setDisplayedWhenStopped(false);
        spinner.setAccessibilityIdentifier(Some(&NSString::from_str("profile-setup.waiting")));
        Layout::prepare(&spinner);
        Layout::width(&spinner, 24.0);
        Layout::height(&spinner, 24.0);
        let key_page = Self::stack(&[&title, &explanation, pin.view(), &spinner], mtm);
        key_page.setAlignment(NSLayoutAttribute::CenterX);
        key_page.setSpacing(16.0);
        root.addSubview(&key_page);
        Layout::prepare(&key_page);
        key_page
            .topAnchor()
            .constraintEqualToAnchor(&root.topAnchor())
            .setActive(true);
        key_page
            .leadingAnchor()
            .constraintEqualToAnchor(&root.leadingAnchor())
            .setActive(true);
        key_page
            .trailingAnchor()
            .constraintEqualToAnchor(&root.trailingAnchor())
            .setActive(true);
        for view in [&*title as &NSView, &*explanation, pin.view()] {
            Layout::fill_width(view, &key_page);
        }
        Self {
            root,
            pin_request,
            qr_page,
            qr_content,
            qr_value: RefCell::new(None),
            qr_view: RefCell::new(None),
            qr_instruction,
            key_page,
            title,
            explanation,
            pin,
            spinner,
            pin_submit,
            use_security_key,
            use_security_key_target,
            mtm,
        }
    }

    pub(crate) fn required_height(&self) -> f64 {
        if !self.qr_page.isHidden() {
            self.qr_page.fittingSize().height
        } else {
            self.key_page.fittingSize().height
        }
    }

    pub(crate) fn pin_submit(&self) -> &NSButton {
        &self.pin_submit
    }

    pub(crate) fn render(
        &self,
        presentation: &ProfileSetupInteractionPresentation,
        request_id: Option<keyringd_fido_client::InteractionId>,
    ) {
        let request_id = if matches!(presentation.screen, ProfileSetupInteractionScreen::Pin(_)) {
            request_id
        } else {
            None
        };
        if self.pin_request.replace(request_id) != request_id {
            self.pin.reset();
        }
        let visible = !matches!(presentation.screen, ProfileSetupInteractionScreen::Hidden);
        self.root.setHidden(!visible);
        self.pin_submit.setHidden(true);
        self.pin_submit.setKeyEquivalent(&NSString::new());
        self.use_security_key.setHidden(true);
        unsafe { self.spinner.stopAnimation(None) };
        if !visible {
            self.pin.render(None);
            return;
        }
        if matches!(
            presentation.screen,
            ProfileSetupInteractionScreen::Cancelling
        ) {
            self.render_cancelling(presentation);
            return;
        }
        let screen = &presentation.screen;
        let on_qr = matches!(screen, ProfileSetupInteractionScreen::Qr(_));
        let on_pin = matches!(screen, ProfileSetupInteractionScreen::Pin(_));
        self.qr_page.setHidden(!on_qr);
        self.key_page.setHidden(on_qr);
        self.title.setHidden(on_pin);
        self.pin.render(presentation.pin_prompt().as_ref());
        self.spinner.setHidden(on_pin);
        self.pin_submit.setHidden(!on_pin);
        if on_pin {
            self.pin_submit.setKeyEquivalent(&NSString::from_str("\r"));
        }
        self.use_security_key_target.set_event(
            presentation
                .operation_id
                .map(ProfileSetupEvent::UseSecurityKeyRequested),
        );
        match screen {
            ProfileSetupInteractionScreen::Qr(hybrid) => {
                let presentation = HybridQrPresentation::new(
                    &hybrid.value,
                    &hybrid.context,
                    hybrid.security_key_available,
                    hybrid.unavailable,
                );
                self.qr_instruction
                    .setStringValue(&NSString::from_str(presentation.instruction()));
                if self.qr_value.borrow().as_ref() != Some(&hybrid.value) {
                    self.qr_value.replace(None);
                    for view in self.qr_content.subviews() {
                        view.removeFromSuperview();
                    }
                    let qr = HybridQrView::new(&presentation, self.mtm);
                    self.qr_content.addSubview(qr.view());
                    Layout::pin(qr.view(), &self.qr_content, 0.0, 0.0, 0.0, 0.0);
                    self.qr_view.replace(Some(qr));
                    self.qr_value.replace(Some(hybrid.value.clone()));
                }
                self.use_security_key
                    .setHidden(!presentation.security_key_available);
                if let Some(qr) = self.qr_view.borrow().as_ref() {
                    qr.render(&presentation);
                }
            }
            ProfileSetupInteractionScreen::Pin(_) => {
                self.render_copy(presentation);
            }
            ProfileSetupInteractionScreen::Waiting { .. } => {
                self.render_copy(presentation);
                unsafe { self.spinner.startAnimation(None) };
            }
            ProfileSetupInteractionScreen::Preparing => {
                self.render_copy(presentation);
                unsafe { self.spinner.startAnimation(None) };
            }
            ProfileSetupInteractionScreen::Hidden | ProfileSetupInteractionScreen::Cancelling => {
                unreachable!("non-interactive presentations were handled above")
            }
        }
    }

    fn render_cancelling(&self, presentation: &ProfileSetupInteractionPresentation) {
        self.root.setHidden(false);
        self.pin_submit.setHidden(true);
        self.pin_submit.setKeyEquivalent(&NSString::new());
        self.use_security_key.setHidden(true);
        self.qr_page.setHidden(true);
        self.key_page.setHidden(false);
        self.title.setHidden(false);
        self.pin.render(None);
        self.spinner.setHidden(false);
        self.title
            .setStringValue(&NSString::from_str(&presentation.title()));
        self.explanation
            .setStringValue(&NSString::from_str(&presentation.description()));
        unsafe { self.spinner.startAnimation(None) };
    }

    fn render_copy(&self, presentation: &ProfileSetupInteractionPresentation) {
        self.title
            .setStringValue(&NSString::from_str(&presentation.title()));
        self.explanation
            .setStringValue(&NSString::from_str(&presentation.description()));
    }

    fn label(size: f64, bold: bool, mtm: MainThreadMarker) -> Retained<NSTextField> {
        let label = NSTextField::wrappingLabelWithString(&NSString::new(), mtm);
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

    fn stack(views: &[&NSView], mtm: MainThreadMarker) -> Retained<NSStackView> {
        let stack = NSStackView::stackViewWithViews(&NSArray::from_slice(views), mtm);
        stack.setOrientation(NSUserInterfaceLayoutOrientation::Vertical);
        stack.setAlignment(NSLayoutAttribute::Leading);
        stack
    }
}
