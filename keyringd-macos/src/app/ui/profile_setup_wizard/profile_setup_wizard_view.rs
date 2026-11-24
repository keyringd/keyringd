use keyringd_ui_core::profile_setup_window::{
    ProfileSetupInteractionScreen, ProfileSetupWindowState,
};
use objc2::rc::Retained;
use objc2::{MainThreadMarker, sel};
use objc2_app_kit::{
    NSAccessibility, NSButton, NSImage, NSLayoutConstraint, NSTextAlignment, NSTextField,
    NSUserInterfaceLayoutOrientation, NSView,
};
use objc2_foundation::NSString;

use keyringd_ui_core::runtime::UiEventTarget;
use keyringd_ui_core::text as ui_text;

use super::{EditingPage, ProfileSetupActionTarget, ProfileSetupEvent, ProfileSetupScreen};
use crate::app::ui::Layout;
use crate::app::ui::profile_setup_interaction::ProfileSetupInteractionPage;
use crate::app::ui::profile_setup_wizard::ProfileFormView;
use crate::app::ui::profile_setup_wizard::{
    PasskeyNotFoundPage, PasskeySelectionPage, ProfileSetupControls,
};

/// Owns one visible page, its shared heading and its navigation actions.
pub(crate) struct ProfileSetupWizardView {
    body_minimum_height: Retained<objc2_app_kit::NSLayoutConstraint>,
    form: ProfileFormView,
    selection: PasskeySelectionPage,
    heading: Retained<NSTextField>,
    heading_top: Retained<NSLayoutConstraint>,
    intro: Retained<NSTextField>,
    interaction: ProfileSetupInteractionPage,
    trust_entries: Retained<NSButton>,
    submit: Retained<NSButton>,
    cancel: Retained<NSButton>,
    not_found: PasskeyNotFoundPage,
    back: Retained<NSButton>,
    not_found_continue: Retained<NSButton>,
    credential_continue: Retained<NSButton>,
    done_button: Retained<NSButton>,
    _targets: Vec<Retained<ProfileSetupActionTarget>>,
}

impl ProfileSetupWizardView {
    pub(crate) fn new(
        content: &NSView,
        events: UiEventTarget<ProfileSetupEvent>,
        mtm: MainThreadMarker,
    ) -> Self {
        let heading = ProfileSetupControls::label(ui_text::PROFILE_SETUP_TITLE, 20.0, true, mtm);
        heading.setAccessibilityIdentifier(Some(&NSString::from_str("profile-setup.heading")));
        content.addSubview(&heading);
        Layout::prepare(&heading);
        let heading_top = heading
            .topAnchor()
            .constraintEqualToAnchor_constant(&content.topAnchor(), 28.0);
        heading_top.setActive(true);
        heading
            .leadingAnchor()
            .constraintEqualToAnchor_constant(&content.leadingAnchor(), 36.0)
            .setActive(true);
        heading
            .trailingAnchor()
            .constraintEqualToAnchor_constant(&content.trailingAnchor(), -36.0)
            .setActive(true);
        let intro = ProfileSetupControls::label("", 13.0, false, mtm);
        intro.setAccessibilityIdentifier(Some(&NSString::from_str("profile-setup.purpose")));
        content.addSubview(&intro);
        Layout::prepare(&intro);
        intro
            .topAnchor()
            .constraintEqualToAnchor_constant(&heading.bottomAnchor(), 10.0)
            .setActive(true);
        intro
            .leadingAnchor()
            .constraintEqualToAnchor(&heading.leadingAnchor())
            .setActive(true);
        intro
            .trailingAnchor()
            .constraintEqualToAnchor(&heading.trailingAnchor())
            .setActive(true);

        let mut targets = Vec::new();
        let mut button = |title: &str, id: &str, event: ProfileSetupEvent| {
            let target = ProfileSetupActionTarget::new(mtm, events.clone(), event);
            let button = unsafe {
                NSButton::buttonWithTitle_target_action(
                    &NSString::from_str(title),
                    Some(&target),
                    Some(sel!(performAction:)),
                    mtm,
                )
            };
            button.setAccessibilityIdentifier(Some(&NSString::from_str(id)));
            button
                .widthAnchor()
                .constraintGreaterThanOrEqualToConstant(80.0)
                .setActive(true);
            Layout::height(&button, 32.0);
            targets.push(target);
            button
        };
        let submit = button(
            ui_text::PROFILE_SETUP_CREATE,
            "profile-setup.create",
            ProfileSetupEvent::SetupRequested,
        );
        let cancel = button(
            ui_text::CANCEL,
            "profile-setup.cancel",
            ProfileSetupEvent::Dismissed,
        );
        let not_found_continue = button(
            ui_text::CONTINUE,
            "profile-setup.not-found-continue",
            ProfileSetupEvent::PasskeyNotFoundChoiceConfirmed,
        );
        let credential_continue = button(
            ui_text::CONTINUE,
            "profile-setup.credential-continue",
            ProfileSetupEvent::CredentialSelectionConfirmed,
        );
        let trust_entries = button(
            ui_text::TRUST_ENTRIES_MENU_ACTION,
            "profile-setup.trust-entries",
            ProfileSetupEvent::ShowTrustEntries,
        );
        let done_button = button(
            ui_text::DONE,
            "profile-setup.done",
            ProfileSetupEvent::Dismissed,
        );
        let back_target =
            ProfileSetupActionTarget::new(mtm, events.clone(), ProfileSetupEvent::BackRequested);
        let back_label = NSString::from_str(ui_text::BACK);
        let chevron = NSImage::imageWithSystemSymbolName_accessibilityDescription(
            &NSString::from_str("chevron.backward"),
            Some(&back_label),
        )
        .expect("AppKit provides the standard back symbol");
        let back = unsafe {
            NSButton::buttonWithImage_target_action(
                &chevron,
                Some(&back_target),
                Some(sel!(performAction:)),
                mtm,
            )
        };
        back.setBordered(false);
        back.setAccessibilityLabel(Some(&back_label));
        back.setAccessibilityIdentifier(Some(&NSString::from_str("profile-setup.back")));
        back.setToolTip(Some(&back_label));
        targets.push(back_target);
        content.addSubview(&back);
        Layout::prepare(&back);
        Layout::width(&back, 32.0);
        Layout::height(&back, 32.0);
        back.leadingAnchor()
            .constraintEqualToAnchor_constant(&heading.leadingAnchor(), -8.0)
            .setActive(true);
        back.topAnchor()
            .constraintEqualToAnchor_constant(&content.topAnchor(), 8.0)
            .setActive(true);
        let actions = ProfileSetupControls::stack(
            &[
                &cancel,
                &submit,
                &not_found_continue,
                &credential_continue,
                &trust_entries,
                &done_button,
            ],
            NSUserInterfaceLayoutOrientation::Horizontal,
            mtm,
        );
        actions.setSpacing(8.0);
        content.addSubview(&actions);
        Layout::prepare(&actions);
        actions
            .trailingAnchor()
            .constraintEqualToAnchor_constant(&content.trailingAnchor(), -36.0)
            .setActive(true);
        actions
            .bottomAnchor()
            .constraintEqualToAnchor_constant(&content.bottomAnchor(), -24.0)
            .setActive(true);
        let body = NSView::new(mtm);
        content.addSubview(&body);
        Layout::prepare(&body);
        body.topAnchor()
            .constraintEqualToAnchor_constant(&intro.bottomAnchor(), 16.0)
            .setActive(true);
        body.leadingAnchor()
            .constraintEqualToAnchor(&heading.leadingAnchor())
            .setActive(true);
        body.trailingAnchor()
            .constraintEqualToAnchor(&heading.trailingAnchor())
            .setActive(true);
        body.bottomAnchor()
            .constraintEqualToAnchor_constant(&actions.topAnchor(), -24.0)
            .setActive(true);

        let body_minimum_height = body
            .heightAnchor()
            .constraintGreaterThanOrEqualToConstant(0.0);
        body_minimum_height.setActive(true);
        let form = ProfileFormView::new(&body, events.clone(), mtm);

        let not_found = PasskeyNotFoundPage::new(&body, events.clone(), mtm);

        let selection = PasskeySelectionPage::new(&body, events.clone(), mtm);
        let interaction = ProfileSetupInteractionPage::new(&body, events, mtm);
        actions.insertArrangedSubview_atIndex(interaction.pin_submit(), 1);
        Self {
            body_minimum_height,
            form,
            selection,
            heading,
            heading_top,
            intro,
            interaction,
            trust_entries,
            submit,
            cancel,
            not_found,
            back,
            not_found_continue,
            credential_continue,
            done_button,
            _targets: targets,
        }
    }

    pub(crate) fn reset_input(&self) {
        self.form.reset();
    }

    pub(crate) fn render(&self, state: &ProfileSetupWindowState) {
        let screen = state.screen();
        let presentation = state.interaction_presentation();
        let entering_pin = matches!(presentation.screen, ProfileSetupInteractionScreen::Pin(_));
        let show_back = state.can_go_back() && !entering_pin;
        let align = if entering_pin {
            NSTextAlignment::Center
        } else {
            NSTextAlignment::Left
        };
        self.heading.setAlignment(align);
        self.intro.setAlignment(align);
        self.heading_top
            .setConstant(if show_back { 52.0 } else { 28.0 });
        if screen == ProfileSetupScreen::Cancelling {
            self.form.view().setHidden(true);
            self.not_found.view().setHidden(true);
            self.selection.view().setHidden(true);
            self.interaction
                .render(&presentation, state.pin_request_id());
            self.header(ui_text::PROFILE_SETUP_CANCELLING, "");
            self.hide_actions();
            self.body_minimum_height
                .setConstant(self.interaction.required_height());
            return;
        }
        self.form.render(state);
        self.form
            .view()
            .setHidden(screen != ProfileSetupScreen::Form);
        self.not_found
            .view()
            .setHidden(screen != ProfileSetupScreen::PasskeyNotFound);
        self.selection
            .view()
            .setHidden(screen != ProfileSetupScreen::Credentials);
        self.interaction
            .render(&presentation, state.pin_request_id());

        self.hide_actions();
        self.back.setHidden(!show_back);
        match screen {
            ProfileSetupScreen::Cancelling => unreachable!("cancellation was rendered above"),
            ProfileSetupScreen::Form => {
                self.header(
                    ui_text::PROFILE_SETUP_TITLE,
                    ui_text::PROFILE_SETUP_FORM_DESCRIPTION,
                );
                self.cancel.setHidden(false);
                self.cancel.setKeyEquivalent(&NSString::from_str("\u{1b}"));
                self.submit.setHidden(false);
                self.submit
                    .setTitle(&NSString::from_str(state.submit_label()));
                self.submit.setEnabled(state.can_start());
                self.submit.setKeyEquivalent(&NSString::from_str("\r"));
            }
            ProfileSetupScreen::PasskeyNotFound => {
                self.header(
                    ui_text::PROFILE_SETUP_NOT_FOUND_TITLE,
                    ui_text::PROFILE_SETUP_NOT_FOUND,
                );
                if let Some(EditingPage::PasskeyNotFound(choice)) = state.editing_page() {
                    self.not_found.render(choice);
                }
                self.cancel.setHidden(false);
                self.cancel.setKeyEquivalent(&NSString::from_str("\u{1b}"));
                self.back.setHidden(false);
                self.not_found_continue.setHidden(false);
                self.not_found_continue
                    .setKeyEquivalent(&NSString::from_str("\r"));
            }
            ProfileSetupScreen::Credentials => {
                self.header(
                    ui_text::PROFILE_SETUP_CHOOSE_PASSKEY,
                    ui_text::PROFILE_SETUP_CHOOSE_PASSKEY_DESCRIPTION,
                );
                if let Some(selection) = state.credential_selection() {
                    self.selection.render(selection);
                }
                self.cancel.setHidden(false);
                self.cancel.setKeyEquivalent(&NSString::from_str("\u{1b}"));
                self.credential_continue.setHidden(false);
                self.credential_continue
                    .setKeyEquivalent(&NSString::from_str("\r"));
            }
            ProfileSetupScreen::Interaction => {
                self.header(
                    &presentation.stage_title(),
                    &presentation.stage_description(),
                );
                self.cancel.setHidden(false);
                self.cancel.setKeyEquivalent(&NSString::from_str("\u{1b}"));
            }
            ProfileSetupScreen::Completed => {
                if let Some(profile) = state.completed_profile() {
                    self.header(
                        ui_text::PROFILE_SETUP_COMPLETED_TITLE,
                        &keyringd_ui_core::profile_setup_window::completed_description(
                            profile.display_name(),
                        ),
                    );
                }
                self.trust_entries.setHidden(false);
                self.done_button.setHidden(false);
                self.done_button.setKeyEquivalent(&NSString::from_str("\r"));
            }
        }
        let height = match screen {
            ProfileSetupScreen::Form => self.form.view().fittingSize().height,
            ProfileSetupScreen::PasskeyNotFound => self.not_found.view().fittingSize().height,
            ProfileSetupScreen::Interaction => self.interaction.required_height(),
            // The candidate list scrolls inside the retained viewport.
            _ => 0.0,
        };
        self.body_minimum_height.setConstant(height);
    }

    fn hide_actions(&self) {
        for button in [
            &self.submit,
            &self.cancel,
            &self.back,
            &self.not_found_continue,
            &self.credential_continue,
            &self.trust_entries,
            &self.done_button,
        ] {
            button.setHidden(true);
            button.setKeyEquivalent(&NSString::new());
        }
    }

    fn header(&self, title: &str, purpose: &str) {
        self.heading.setStringValue(&NSString::from_str(title));
        self.intro.setStringValue(&NSString::from_str(purpose));
    }
}
