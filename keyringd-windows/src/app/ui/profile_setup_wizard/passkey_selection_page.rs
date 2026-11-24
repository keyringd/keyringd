use keyringd_ui_core::text as ui_text;

use windows_reactor::{Element, text_block};

use keyringd_ui_core::profile_setup_window::PasskeyCredentialSelection;
use keyringd_ui_core::{profile_setup_window::ProfileSetupEvent, runtime::UiEventTarget};
use windows_reactor::{AccessibilityExt, RadioButtons, vstack};

pub(super) struct PasskeySelectionPage;

impl PasskeySelectionPage {
    pub(super) fn render(
        selection: PasskeyCredentialSelection,
        events: UiEventTarget<ProfileSetupEvent>,
    ) -> Element {
        vstack((
            text_block(ui_text::PROFILE_SETUP_CHOOSE_PASSKEY)
                .semibold()
                .wrap(),
            text_block(ui_text::PROFILE_SETUP_CHOOSE_PASSKEY_DESCRIPTION).wrap(),
            RadioButtons::new(
                selection
                    .candidates()
                    .iter()
                    .map(|candidate| candidate.label().to_owned())
                    .collect::<Vec<_>>(),
            )
            .automation_id("profile-setup-passkey-candidates")
            .selected_index(selection.selected_index() as i32)
            .on_selection_changed(move |index| {
                if let Ok(index) = usize::try_from(index) {
                    events.send(ProfileSetupEvent::CredentialChoiceChanged(index));
                }
            }),
        ))
        .spacing(12.0)
        .automation_id("profile-setup-passkey-selection")
        .into()
    }
}
