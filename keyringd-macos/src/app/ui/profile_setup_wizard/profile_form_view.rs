use crate::app::ui::Layout;
use crate::app::ui::profile_setup_wizard::ProfileSetupActionTarget;
use crate::app::ui::profile_setup_wizard::ProfileSetupControls;
use keyringd_ssh_agent_protocol::SshSignatureAlgorithm;

use keyringd_ui_core::text as ui_text;
use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2::{MainThreadMarker, sel};
use objc2_app_kit::{
    NSAccessibility, NSButton, NSColor, NSControlStateValueOff, NSControlStateValueOn, NSFont,
    NSStackView, NSTextField, NSUserInterfaceLayoutOrientation, NSView,
};
use objc2_foundation::NSString;

use keyringd_ui_core::runtime::UiEventTarget;

use crate::app::ui::profile_setup_wizard::ProfileSetupEvent;
use keyringd_core::application::PasskeyChoice;
use keyringd_ui_core::profile_setup_window::ProfileSetupWindowState;

use super::ProfileNameFieldDelegate;

pub(crate) struct ProfileFormView {
    root: Retained<NSStackView>,
    status: Retained<NSTextField>,
    _targets: Vec<Retained<ProfileSetupActionTarget>>,
    name: Retained<NSTextField>,
    create_new: Retained<NSButton>,
    recover: Retained<NSButton>,
    sk_format: Retained<NSButton>,
    ecdsa_format: Retained<NSButton>,
    advanced: Retained<NSButton>,
    format_group: Retained<NSStackView>,
    _name_delegate: Retained<ProfileNameFieldDelegate>,
}

impl ProfileFormView {
    pub(crate) fn new(
        parent: &NSView,
        events: UiEventTarget<ProfileSetupEvent>,
        mtm: MainThreadMarker,
    ) -> Self {
        let name_label = ProfileSetupControls::label(ui_text::PROFILE_SETUP_NAME, 13.0, true, mtm);
        let name = NSTextField::new(mtm);
        name.setFont(Some(&NSFont::systemFontOfSize(13.0)));
        name.setAccessibilityIdentifier(Some(&NSString::from_str("profile-setup.name")));
        name.setPlaceholderString(Some(&NSString::from_str(
            ui_text::PROFILE_SETUP_NAME_PLACEHOLDER,
        )));
        Layout::height(&name, 28.0);
        let passkey_label =
            ProfileSetupControls::label(ui_text::PROFILE_SETUP_PASSKEY, 13.0, true, mtm);
        let (create_new, create_target) = ProfileSetupControls::radio(
            ui_text::PROFILE_SETUP_CREATE_PASSKEY,
            "profile-setup.create-new-passkey",
            ProfileSetupEvent::PasskeyChanged(PasskeyChoice::CreateNew),
            events.clone(),
            mtm,
        );
        let (recover, recover_target) = ProfileSetupControls::radio(
            ui_text::PROFILE_SETUP_EXISTING_PASSKEY,
            "profile-setup.use-existing-passkey",
            ProfileSetupEvent::PasskeyChanged(PasskeyChoice::Existing),
            events.clone(),
            mtm,
        );
        let advanced_target = ProfileSetupActionTarget::new(
            mtm,
            events.clone(),
            ProfileSetupEvent::AdvancedSettingsToggled,
        );
        let advanced = unsafe {
            NSButton::buttonWithTitle_target_action(
                &NSString::from_str(ui_text::PROFILE_SETUP_ADVANCED_COLLAPSED),
                Some(&advanced_target),
                Some(sel!(performAction:)),
                mtm,
            )
        };
        advanced.setAccessibilityIdentifier(Some(&NSString::from_str(
            "profile-setup.advanced-settings",
        )));
        let format_label =
            ProfileSetupControls::label(ui_text::PROFILE_SETUP_SIGNATURE_FORMAT, 13.0, true, mtm);
        let format_description = ProfileSetupControls::label(
            ui_text::PROFILE_SETUP_SIGNATURE_DESCRIPTION,
            12.0,
            false,
            mtm,
        );
        let (sk_format, sk_target) = ProfileSetupControls::radio(
            ui_text::PROFILE_SETUP_SECURITY_KEY_FORMAT,
            "profile-setup.sk-format",
            ProfileSetupEvent::SignatureAlgorithmChanged(
                SshSignatureAlgorithm::SkEcdsaSha2NistP256,
            ),
            events.clone(),
            mtm,
        );
        let sk_description = ProfileSetupControls::label(
            ui_text::PROFILE_SETUP_SECURITY_KEY_DESCRIPTION,
            12.0,
            false,
            mtm,
        );
        let (ecdsa_format, ecdsa_target) = ProfileSetupControls::radio(
            ui_text::PROFILE_SETUP_ECDSA_FORMAT,
            "profile-setup.ecdsa-format",
            ProfileSetupEvent::SignatureAlgorithmChanged(SshSignatureAlgorithm::EcdsaSha2NistP256),
            events.clone(),
            mtm,
        );
        let ecdsa_description =
            ProfileSetupControls::label(ui_text::PROFILE_SETUP_ECDSA_DESCRIPTION, 12.0, false, mtm);
        let format_group = ProfileSetupControls::stack(
            &[
                &format_label,
                &format_description,
                &ecdsa_format,
                &ecdsa_description,
                &sk_format,
                &sk_description,
            ],
            NSUserInterfaceLayoutOrientation::Vertical,
            mtm,
        );
        format_group.setSpacing(8.0);
        format_group.setHidden(true);
        let targets = vec![
            create_target,
            recover_target,
            advanced_target,
            sk_target,
            ecdsa_target,
        ];
        let status = ProfileSetupControls::label("", 13.0, false, mtm);
        status.setAccessibilityIdentifier(Some(&NSString::from_str("profile-setup.status")));
        let form_container = ProfileSetupControls::stack(
            &[
                &name_label,
                &name,
                &passkey_label,
                &create_new,
                &recover,
                &advanced,
                &format_group,
                &status,
            ],
            NSUserInterfaceLayoutOrientation::Vertical,
            mtm,
        );
        form_container.setSpacing(12.0);
        parent.addSubview(&form_container);
        Layout::pin_top(&form_container, parent);
        for view in [&*name as &NSView, &*status] {
            Layout::fill_width(view, &form_container);
        }
        let name_delegate = ProfileNameFieldDelegate::new(mtm, name.clone(), events);
        unsafe {
            name.setDelegate(Some(ProtocolObject::from_ref(&*name_delegate)));
        }
        Self {
            root: form_container,
            status,
            _targets: targets,
            name,
            create_new,
            recover,
            sk_format,
            ecdsa_format,
            advanced,
            format_group,
            _name_delegate: name_delegate,
        }
    }

    pub(crate) fn view(&self) -> &NSView {
        &self.root
    }

    pub(crate) fn reset(&self) {
        self.name.setStringValue(&objc2_foundation::NSString::new());
        if let Some(window) = self.name.window() {
            window.makeFirstResponder(Some(&self.name));
        }
    }

    pub(crate) fn render(&self, state: &ProfileSetupWindowState) {
        self.status.setStringValue(&NSString::from_str(
            state
                .notice_message()
                .unwrap_or(ui_text::PROFILE_SETUP_FORM_HINT),
        ));
        let status_color = if state.notice().is_some_and(|notice| notice.is_error()) {
            NSColor::systemRedColor()
        } else {
            NSColor::secondaryLabelColor()
        };
        self.status.setTextColor(Some(&status_color));

        let editable = !state.is_running() && state.completed_profile().is_none();
        self.name.setEnabled(editable);
        self.create_new.setEnabled(editable);
        self.recover.setEnabled(editable);
        self.advanced.setEnabled(editable);
        self.sk_format.setEnabled(editable);
        self.ecdsa_format.setEnabled(editable);
        self.advanced
            .setTitle(&NSString::from_str(if state.advanced_settings() {
                ui_text::PROFILE_SETUP_ADVANCED_EXPANDED
            } else {
                ui_text::PROFILE_SETUP_ADVANCED_COLLAPSED
            }));
        self.format_group.setHidden(!state.advanced_settings());
        self.sk_format.setState(
            if state.signature_algorithm() == SshSignatureAlgorithm::SkEcdsaSha2NistP256 {
                NSControlStateValueOn
            } else {
                NSControlStateValueOff
            },
        );
        self.ecdsa_format.setState(
            if state.signature_algorithm() == SshSignatureAlgorithm::EcdsaSha2NistP256 {
                NSControlStateValueOn
            } else {
                NSControlStateValueOff
            },
        );
        self.create_new
            .setState(if state.passkey() == PasskeyChoice::CreateNew {
                NSControlStateValueOn
            } else {
                NSControlStateValueOff
            });
        self.recover
            .setState(if state.passkey() == PasskeyChoice::Existing {
                NSControlStateValueOn
            } else {
                NSControlStateValueOff
            });
    }
}
