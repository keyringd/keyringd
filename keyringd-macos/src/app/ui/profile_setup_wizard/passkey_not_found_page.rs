use super::ProfileSetupControls;
use super::{PasskeyNotFoundChoice, ProfileSetupActionTarget, ProfileSetupEvent};
use crate::app::ui::Layout;
use keyringd_ui_core::text as ui_text;
use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2_app_kit::{
    NSButton, NSControlStateValueOff, NSControlStateValueOn, NSStackView,
    NSUserInterfaceLayoutOrientation, NSView,
};

use keyringd_ui_core::runtime::UiEventTarget;

pub(crate) struct PasskeyNotFoundPage {
    root: Retained<NSStackView>,
    try_another: Retained<NSButton>,
    create_new_passkey: Retained<NSButton>,
    _targets: Vec<Retained<ProfileSetupActionTarget>>,
}
impl PasskeyNotFoundPage {
    pub(crate) fn new(
        parent: &NSView,
        events: UiEventTarget<ProfileSetupEvent>,
        mtm: MainThreadMarker,
    ) -> Self {
        let (try_another, another_target) = ProfileSetupControls::radio(
            ui_text::PROFILE_SETUP_TRY_ANOTHER_DEVICE,
            "profile-setup.try-another-device",
            ProfileSetupEvent::PasskeyNotFoundChoiceChanged(
                PasskeyNotFoundChoice::TryAnotherDevice,
            ),
            events.clone(),
            mtm,
        );
        let (create_new_passkey, new_target) = ProfileSetupControls::radio(
            ui_text::PROFILE_SETUP_CREATE_INSTEAD,
            "profile-setup.create-new-passkey-instead",
            ProfileSetupEvent::PasskeyNotFoundChoiceChanged(
                PasskeyNotFoundChoice::CreateNewPasskey,
            ),
            events.clone(),
            mtm,
        );
        let targets = vec![another_target, new_target];
        let recovery_warning =
            ProfileSetupControls::label(ui_text::PROFILE_SETUP_RECOVERY_WARNING, 13.0, false, mtm);
        let not_found_container = ProfileSetupControls::stack(
            &[&try_another, &create_new_passkey, &recovery_warning],
            NSUserInterfaceLayoutOrientation::Vertical,
            mtm,
        );
        not_found_container.setSpacing(16.0);
        recovery_warning
            .widthAnchor()
            .constraintEqualToAnchor(&not_found_container.widthAnchor())
            .setActive(true);
        parent.addSubview(&not_found_container);
        Layout::pin_top(&not_found_container, parent);

        Self {
            root: not_found_container,
            try_another,
            create_new_passkey,
            _targets: targets,
        }
    }
    pub(crate) fn view(&self) -> &NSView {
        &self.root
    }
    pub(crate) fn render(&self, choice: PasskeyNotFoundChoice) {
        self.try_another
            .setState(if choice == PasskeyNotFoundChoice::TryAnotherDevice {
                NSControlStateValueOn
            } else {
                NSControlStateValueOff
            });
        self.create_new_passkey
            .setState(if choice == PasskeyNotFoundChoice::CreateNewPasskey {
                NSControlStateValueOn
            } else {
                NSControlStateValueOff
            });
    }
}
