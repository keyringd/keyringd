use super::{CompletionPage, PasskeyNotFoundPage, PasskeySelectionPage, ProfileFormPage};
use crate::app::ui::profile_setup_interaction::ProfileSetupInteractionPage;
use gtk4::glib::object::{Cast, ObjectExt};
use gtk4::prelude::{AccessibleExtManual, ButtonExt, CheckButtonExt, EditableExt, WidgetExt};
use keyringd_core::application::PasskeyChoice;
use keyringd_ssh_agent_protocol::SshSignatureAlgorithm;

use keyringd_ui_core::profile_setup_window::ProfileSetupWindowState;
use keyringd_ui_core::profile_setup_window::{EditingPage, ProfileSetupEvent, ProfileSetupNotice};
use keyringd_ui_core::runtime::UiEventTarget;
use keyringd_ui_core::text as ui_text;
use libadwaita::prelude::EntryRowExt;
use std::cell::Cell;
use std::rc::Rc;

pub(in crate::app::ui) struct ProfileSetupWizardView {
    root: libadwaita::ToolbarView,
    back: gtk4::Button,
    form: ProfileFormPage,
    completion: CompletionPage,
    stack: gtk4::Stack,
    not_found: PasskeyNotFoundPage,
    selection: PasskeySelectionPage,
    interaction: ProfileSetupInteractionPage,
    rendering: Rc<Cell<bool>>,
}

impl ProfileSetupWizardView {
    pub(in crate::app::ui) fn new(events: UiEventTarget<ProfileSetupEvent>) -> Self {
        let form = ProfileFormPage::render();
        let completion = CompletionPage::render();
        let selection = PasskeySelectionPage::new(events.clone());
        let not_found = PasskeyNotFoundPage::render(events.clone());
        let interaction = ProfileSetupInteractionPage::new(events.clone());
        let stack = Self::page_stack();
        for page in [
            &form.root,
            &completion.root,
            selection.widget(),
            &not_found.root,
            interaction.widget().upcast_ref(),
        ] {
            stack.add_child(page);
        }
        stack.set_visible_child(&form.root);
        let back = gtk4::Button::builder()
            .icon_name("go-previous-symbolic")
            .tooltip_text(ui_text::BACK)
            .build();
        back.update_property(&[gtk4::accessible::Property::Label(ui_text::BACK)]);
        let header = libadwaita::HeaderBar::new();
        header.pack_start(&back);
        header.set_title_widget(Some(&libadwaita::WindowTitle::new(
            ui_text::PROFILE_SETUP_TITLE,
            "",
        )));
        let root = libadwaita::ToolbarView::new();
        root.add_top_bar(&header);
        root.set_content(Some(&stack));
        let rendering = Rc::new(Cell::new(false));
        let name_events = events.clone();
        form.name.connect_changed(move |name| {
            name_events.send(ProfileSetupEvent::NameChanged(name.text().to_string()))
        });
        let create = form.create.downgrade();
        form.name.connect_entry_activated(move |_| {
            if let Some(create) = create.upgrade() {
                create.emit_clicked();
            }
        });
        for (button, choice) in [
            (form.create_passkey.widget(), PasskeyChoice::CreateNew),
            (form.existing_passkey.widget(), PasskeyChoice::Existing),
        ] {
            let events = events.clone();
            let rendering = rendering.clone();
            button.connect_toggled(move |button| {
                if button.is_active() && !rendering.get() {
                    events.send(ProfileSetupEvent::PasskeyChanged(choice));
                }
            });
        }
        for (button, format) in [
            (
                form.sk_format.widget(),
                SshSignatureAlgorithm::SkEcdsaSha2NistP256,
            ),
            (
                form.ecdsa_format.widget(),
                SshSignatureAlgorithm::EcdsaSha2NistP256,
            ),
        ] {
            let events = events.clone();
            button.connect_toggled(move |button| {
                if button.is_active() {
                    events.send(ProfileSetupEvent::SignatureAlgorithmChanged(format.clone()));
                }
            });
        }
        let advanced_events = events.clone();
        form.advanced.connect_expanded_notify(move |_| {
            advanced_events.send(ProfileSetupEvent::AdvancedSettingsToggled)
        });
        for (button, event) in [
            (&back, ProfileSetupEvent::BackRequested),
            (&form.create, ProfileSetupEvent::SetupRequested),
            (&form.cancel, ProfileSetupEvent::Dismissed),
            (&completion.done, ProfileSetupEvent::Dismissed),
            (
                &completion.open_trust_entries,
                ProfileSetupEvent::ShowTrustEntries,
            ),
            (
                &selection.continue_button,
                ProfileSetupEvent::CredentialSelectionConfirmed,
            ),
            (&selection.cancel, ProfileSetupEvent::Dismissed),
            (
                &not_found.continue_button,
                ProfileSetupEvent::PasskeyNotFoundChoiceConfirmed,
            ),
            (&not_found.cancel, ProfileSetupEvent::Dismissed),
        ] {
            let events = events.clone();
            button.connect_clicked(move |_| events.send(event.clone()));
        }
        Self {
            root,
            back,
            form,
            completion,
            stack,
            not_found,
            selection,
            interaction,
            rendering,
        }
    }

    fn page_stack() -> gtk4::Stack {
        gtk4::Stack::builder()
            .transition_type(gtk4::StackTransitionType::Crossfade)
            .transition_duration(180)
            .vhomogeneous(true)
            .margin_top(24)
            .margin_bottom(24)
            .margin_start(24)
            .margin_end(24)
            .build()
    }
    pub(in crate::app::ui) fn widget(&self) -> &libadwaita::ToolbarView {
        &self.root
    }
    pub(in crate::app::ui) fn render(&self, state: &ProfileSetupWindowState) {
        let running = state.is_running();
        self.back.set_visible(state.can_go_back());
        self.form.root.set_sensitive(!running);
        self.form.create.set_label(state.submit_label());
        self.form.create.set_sensitive(state.can_start());
        self.rendering.set(true);
        self.form
            .create_passkey
            .widget()
            .set_active(state.passkey() == PasskeyChoice::CreateNew);
        self.form
            .existing_passkey
            .widget()
            .set_active(state.passkey() == PasskeyChoice::Existing);
        self.rendering.set(false);
        let presentation = state.interaction_presentation();
        self.interaction
            .render(&presentation, state.pin_request_id());
        if matches!(
            state.screen(),
            keyringd_ui_core::profile_setup_window::ProfileSetupScreen::Credentials
        ) && let Some(selection) = state.credential_selection()
        {
            self.selection.render(selection);
            self.selection
                .header
                .heading
                .set_label(ui_text::PROFILE_SETUP_CHOOSE_PASSKEY);
            self.selection
                .header
                .body
                .set_label(ui_text::PROFILE_SETUP_CHOOSE_PASSKEY_DESCRIPTION);
            self.show_page(self.selection.widget());
        } else if !matches!(
            presentation.screen,
            keyringd_ui_core::profile_setup_window::ProfileSetupInteractionScreen::Hidden
        ) {
            self.show_page(self.interaction.widget().upcast_ref());
        } else if let Some(EditingPage::PasskeyNotFound(choice)) = state.editing_page() {
            self.not_found.render_choice(choice);
            self.not_found
                .header
                .heading
                .set_label(ui_text::PROFILE_SETUP_NOT_FOUND_TITLE);
            self.not_found
                .header
                .body
                .set_label(ui_text::PROFILE_SETUP_NOT_FOUND);
            self.show_page(&self.not_found.root);
        } else if !matches!(state, ProfileSetupWindowState::Completed { .. }) {
            self.form
                .header
                .heading
                .set_label(ui_text::PROFILE_SETUP_TITLE);
            self.form
                .header
                .body
                .set_label(ui_text::PROFILE_SETUP_FORM_DESCRIPTION);
            self.show_page(&self.form.root);
        }
        let status = state
            .progress()
            .map(keyringd_ui_core::profile_setup_window::progress_message)
            .or_else(|| state.notice().map(ProfileSetupNotice::message));
        if let Some(status) = status {
            self.form.status.set_label(status);
            self.form.status.set_visible(true);
        } else {
            self.form.status.set_visible(false);
        }
        if matches!(
            state.notice(),
            Some(
                ProfileSetupNotice::MissingName
                    | ProfileSetupNotice::OperationFailed(_)
                    | ProfileSetupNotice::PasskeyNotFound
                    | ProfileSetupNotice::NewPasskeyAfterFailedRecovery
            )
        ) {
            self.form.status.add_css_class("error");
        } else {
            self.form.status.remove_css_class("error");
        }
        if let Some(profile) = state.completed_profile() {
            self.completion
                .header
                .heading
                .set_label(ui_text::PROFILE_SETUP_COMPLETED_TITLE);
            self.completion.header.body.set_label(
                &keyringd_ui_core::profile_setup_window::completed_description(
                    profile.display_name(),
                ),
            );
            self.show_page(&self.completion.root);
            self.completion.done.grab_focus();
        }
    }
    fn show_page(&self, page: &gtk4::Widget) {
        if self.stack.visible_child().as_ref() != Some(page) {
            self.stack.set_visible_child(page);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gtk4::prelude::{EditableExt, GtkWindowExt};
    use keyringd_core::application::KeyringdApplication;
    use keyringd_core::application::ProfileSetupRequest;
    use keyringd_core::test_support::{
        MemoryProfileRepository, SoftwareDeviceKey, TestPasskeyService,
    };
    use keyringd_ui_core::key_operation::AuthenticatorPromptPurpose;
    use keyringd_ui_core::profile_setup_window::HybridPrompt;
    use keyringd_ui_core::profile_setup_window::{
        ProfileSetupInteractionPresentation, ProfileSetupInteractionScreen,
    };
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    fn settle_layout() {
        let context = gtk4::glib::MainContext::default();
        let deadline = Instant::now() + Duration::from_millis(300);
        while Instant::now() < deadline {
            while context.pending() {
                context.iteration(false);
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    keyringd_testkit::scenario! {
        group "ProfileSetupWizardView::render" {
            case "Completion does not expand to fit the hidden profile form" {
                verify "size the completion page independently of the hidden form" {
                    let profile = tokio::runtime::Runtime::new().unwrap().block_on(async {
                        let app = KeyringdApplication::new(
                            SoftwareDeviceKey::generating(),
                            TestPasskeyService::new(),
                            Arc::new(MemoryProfileRepository::default()),
                        )
                        .unwrap();
                        app.setup_profile(
                            &Default::default(),
                        &Default::default(),
                            ProfileSetupRequest::new("recovered-phone-pc".into(), PasskeyChoice::CreateNew),
                            |_| {},
                            keyringd_core::test_support::reject_credential_selection,
                        )
                        .await
                        .unwrap();
                        app.profiles().pop().unwrap()
                    });
                    gtk4::test_synced(move || {
                        libadwaita::init().unwrap();
                        let view = ProfileSetupWizardView::new(UiEventTarget::new(|_| {}));
                        view.form.name.set_text("recovered-phone-pc");
                        view.render(&ProfileSetupWindowState::editing());
                        view.interaction.render(
                            &ProfileSetupInteractionPresentation {
                                screen: ProfileSetupInteractionScreen::Qr(HybridPrompt {
                                    operation_id: keyringd_fido_client::OperationId::new(1),
                                    value: "FIDO:/123456789".into(),
                                    context: AuthenticatorPromptPurpose::AuthorizeWithExistingPasskey,
                                    unavailable: None,
                                    security_key_available: false,
                                }),
                                context: Some(AuthenticatorPromptPurpose::AuthorizeWithExistingPasskey),
                                stage: None,
                                operation_id: None,
                                status: "",
                            },
                            None,
                        );
                        view.show_page(view.interaction.widget().upcast_ref());
                        let window = libadwaita::Window::builder()
                            .default_width(crate::app::ui::PASSKEY_DIALOG_WIDTH)
                            .default_height(636)
                            .content(view.widget())
                            .build();
                        window.present();
                        settle_layout();
                        let qr_width = window.width();
                        view.render(&ProfileSetupWindowState::Completed { profile: Box::new(profile), window_error: None });
                        settle_layout();
                        let completed_width = window.width();
                        window.close();
                        assert_eq!(qr_width, crate::app::ui::PASSKEY_DIALOG_WIDTH);
                        assert_eq!(completed_width, qr_width);
                    });
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "ProfileSetupWizardView::render" {
            case "Wizard emits typed inputs without owning application state" {
                verify "emit typed wizard events for form, navigation, and completion input" {
                    gtk4::test_synced(|| {
                        libadwaita::init().unwrap();
                        let events = Rc::new(std::cell::RefCell::new(Vec::new()));
                        let output = events.clone();
                        let view = ProfileSetupWizardView::new(UiEventTarget::new(move |event| {
                            output.borrow_mut().push(event)
                        }));
                        view.render(&ProfileSetupWindowState::editing());
                        assert!(events.borrow().is_empty());
                        view.form.name.set_text("test");
                        view.form.create.emit_clicked();
                        view.back.emit_clicked();
                        assert!(
                            matches!(&events.borrow()[0], ProfileSetupEvent::NameChanged(name) if name == "test")
                        );
                        assert!(matches!(
                            &events.borrow()[1],
                            ProfileSetupEvent::SetupRequested
                        ));
                        assert!(matches!(
                            &events.borrow()[2],
                            ProfileSetupEvent::BackRequested
                        ));
                    });
                }
            }
        }
    }
}
