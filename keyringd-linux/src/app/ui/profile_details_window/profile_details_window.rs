use crate::app::ui::profile_details_window::ProfileDetailsCommandHandler;
use keyringd_ui_core::profile_details_window::ProfileDetailsEvent;
use keyringd_ui_core::profile_details_window::ProfileDetailsState;

use keyringd_ui_core::text as ui_text;

use std::rc::Rc;
use std::sync::Arc;

use gtk4::prelude::{BoxExt, ButtonExt, GtkWindowExt, ObjectExt, WidgetExt};
use keyringd_core::model::ProfileId;
use libadwaita::prelude::{ActionRowExt, MessageDialogExt, PreferencesGroupExt};

use crate::app::LinuxApplication;
use keyringd_ui_core::key_operation::KeyOperationCoordinator;

use crate::app::ui::{ComponentRuntime, UiEventReceiver};

pub(crate) struct ProfileDetailsWindow;

impl ProfileDetailsWindow {
    pub(crate) fn present(
        parent: &gtk4::Window,
        profile_id: ProfileId,
        application: libadwaita::Application,
        profile_application: Arc<LinuxApplication>,
        prompts: Arc<KeyOperationCoordinator>,
    ) {
        let (initial_profiles, mut changes) = UiEventReceiver::from_tokio_watch(
            &tokio::runtime::Handle::current(),
            profile_application.subscribe_profile_changes(),
        );
        let Some(profile) = initial_profiles
            .into_iter()
            .find(|profile| profile.id() == &profile_id)
        else {
            return;
        };
        let state = ProfileDetailsState::new(&profile);
        let runtime = Rc::new(ComponentRuntime::new(state));
        let handler = ProfileDetailsCommandHandler::new(
            application,
            Arc::clone(&profile_application),
            prompts,
            profile_id.clone(),
        );
        let details = libadwaita::PreferencesGroup::new();
        let status_row = detail_row(ui_text::PROFILE_DETAILS_STATUS, runtime.state().status());
        let device_signature_row = detail_row(
            ui_text::PROFILE_DETAILS_SIGNATURE_FORMAT,
            runtime.state().device_key_signature_algorithm(),
        );
        let protection_row = detail_row(
            ui_text::PROFILE_DETAILS_LOCAL_AUTHENTICATION,
            runtime.state().device_key_local_authentication(),
        );
        details.add(&status_row);
        details.add(&device_signature_row);
        details.add(&protection_row);

        let replace = gtk4::Button::builder()
            .label(ui_text::PROFILE_DETAILS_REPLACE)
            .valign(gtk4::Align::Center)
            .build();
        replace.add_css_class("destructive-action");
        let replace_row = libadwaita::ActionRow::builder()
            .title(ui_text::REPLACE_DEVICE_KEY_WINDOW_TITLE)
            .subtitle(
                keyringd_ui_core::profile_details_window::replacement_description("TPM-backed"),
            )
            .build();
        replace_row.add_suffix(&replace);
        let advanced = libadwaita::PreferencesGroup::builder()
            .title(ui_text::PROFILE_DETAILS_ADVANCED)
            .build();
        advanced.add(&replace_row);

        let trust_entries = gtk4::Button::builder()
            .label(ui_text::TRUST_ENTRIES_MENU_ACTION)
            .halign(gtk4::Align::Start)
            .build();
        let related = libadwaita::PreferencesGroup::builder()
            .title(ui_text::PROFILE_DETAILS_ACCESS)
            .build();
        related.add(&trust_entries);

        let content = gtk4::Box::new(gtk4::Orientation::Vertical, 18);
        content.append(&details);
        content.append(&related);
        content.append(&advanced);
        let dialog = crate::app::ui::message_dialog_builder()
            .transient_for(parent)
            .heading(runtime.state().heading())
            .extra_child(&content)
            .close_response("close")
            .build();
        dialog.add_response("close", ui_text::CLOSE);
        dialog.connect_response(None, |dialog, _| dialog.close());
        let runtime_for_trust_entries = Rc::clone(&runtime);
        let handler_for_trust_entries = handler.clone();
        let window_for_trust_entries = dialog.downgrade();
        trust_entries.connect_clicked(move |_| {
            let Some(window) = window_for_trust_entries.upgrade() else {
                return;
            };
            if let Some(command) =
                runtime_for_trust_entries.dispatch(ProfileDetailsEvent::TrustEntriesRequested)
            {
                handler_for_trust_entries.handle(command, &window);
            }
        });
        let runtime_for_replace = Rc::clone(&runtime);
        let handler_for_replace = handler.clone();
        let window_for_replace = dialog.downgrade();
        replace.connect_clicked(move |_| {
            let Some(window) = window_for_replace.upgrade() else {
                return;
            };
            if let Some(command) =
                runtime_for_replace.dispatch(ProfileDetailsEvent::ReplaceDeviceKeyRequested)
            {
                handler_for_replace.handle(command, &window);
            }
        });

        let dialog_for_changes = dialog.downgrade();
        let runtime_for_changes = Rc::clone(&runtime);
        gtk4::glib::MainContext::ref_thread_default().spawn_local(async move {
            while let Some(profiles) = changes.receive().await {
                let Some(dialog) = dialog_for_changes.upgrade() else {
                    break;
                };
                let profile = profiles
                    .iter()
                    .find(|profile| profile.id() == &profile_id)
                    .cloned();
                let event = profile.map_or(ProfileDetailsEvent::ProfileRemoved, |profile| {
                    ProfileDetailsEvent::ProfileChanged(Box::new(profile))
                });
                let command = runtime_for_changes.dispatch_and_render(event, |state| {
                    dialog.set_heading(Some(state.heading()));
                    status_row.set_subtitle(state.status());
                    device_signature_row.set_subtitle(state.device_key_signature_algorithm());
                    protection_row.set_subtitle(state.device_key_local_authentication());
                });
                if let Some(command) = command {
                    handler.handle(command, &dialog);
                    break;
                }
            }
        });
        dialog.present();
    }
}

fn detail_row(title: &str, value: &str) -> libadwaita::ActionRow {
    libadwaita::ActionRow::builder()
        .title(title)
        .subtitle(value)
        .build()
}
