use crate::app::ui::toast::ToastOverlay;
use crate::app::ui::trust_entries_window::TrustEntriesWindowCommandHandler;
use crate::app::ui::trust_entries_window::TrustEntryView;
use keyringd_ui_core::runtime::UiEventTarget;
use keyringd_ui_core::text as ui_text;
use keyringd_ui_core::trust_entries_window::TrustEntriesEvent;
use keyringd_ui_core::trust_entries_window::TrustEntriesState;
use keyringd_ui_core::trust_entries_window::TrustEntryKind;

use gtk4::prelude::{BoxExt, ButtonExt, GtkWindowExt, ObjectExt, WidgetExt};
use keyringd_core::model::ProfileId;
use libadwaita::prelude::{MessageDialogExt, PreferencesGroupExt};
use std::sync::Arc;

use crate::app::LinuxApplication;
use crate::app::ui::ComponentRuntime;
use crate::app::ui::UiEventReceiver;

pub(crate) struct TrustEntriesWindow;

impl TrustEntriesWindow {
    pub(crate) fn present(
        parent: &gtk4::Window,
        profile_application: Arc<LinuxApplication>,
        profile_id: ProfileId,
    ) {
        Self::present_with_notice(parent, profile_application, profile_id, None);
    }

    pub(crate) fn present_with_notice(
        parent: &gtk4::Window,
        profile_application: Arc<LinuxApplication>,
        profile_id: ProfileId,
        notice: Option<String>,
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
        let state = TrustEntriesState::from_profile(&profile).with_notice(notice.clone());
        let runtime = Rc::new(ComponentRuntime::new(state));
        let content = gtk4::Box::new(gtk4::Orientation::Vertical, 18);
        let toast_overlay = ToastOverlay::new();
        let body = gtk4::Box::new(gtk4::Orientation::Vertical, 18);
        if let Some(message) = &notice {
            let banner = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);
            banner.add_css_class("card");
            let icon = gtk4::Image::from_icon_name("dialog-warning-symbolic");
            icon.add_css_class("warning");
            icon.set_valign(gtk4::Align::Start);
            icon.set_margin_top(16);
            icon.set_margin_start(16);
            banner.append(&icon);
            let text = gtk4::Box::new(gtk4::Orientation::Vertical, 6);
            text.set_margin_top(16);
            text.set_margin_bottom(16);
            text.set_margin_end(16);
            let title = gtk4::Label::builder()
                .label(ui_text::TRUST_ENTRIES_REGISTRATION_UPDATE)
                .xalign(0.0)
                .wrap(true)
                .build();
            title.add_css_class("heading");
            text.append(&title);
            text.append(
                &gtk4::Label::builder()
                    .label(message)
                    .xalign(0.0)
                    .wrap(true)
                    .build(),
            );
            banner.append(&text);
            body.append(&banner);
        }
        body.append(&content);
        toast_overlay.widget().set_child(Some(&body));
        let on_toast = toast_overlay.event_target();
        let handler = TrustEntriesWindowCommandHandler::new(on_toast.clone());
        let window_holder = Rc::new(std::cell::RefCell::new(
            None::<gtk4::glib::WeakRef<libadwaita::MessageDialog>>,
        ));
        let runtime_for_events = Rc::clone(&runtime);
        let handler_for_events = handler.clone();
        let window_for_events = Rc::clone(&window_holder);
        let on_event: UiEventTarget<TrustEntriesEvent> = UiEventTarget::new(move |event| {
            let Some(command) = runtime_for_events.dispatch(event) else {
                return;
            };
            if let Some(window) = window_for_events
                .borrow()
                .as_ref()
                .and_then(gtk4::glib::WeakRef::upgrade)
            {
                handler_for_events.handle(command, &window);
            }
        });

        let dialog = libadwaita::MessageDialog::builder()
            .transient_for(parent)
            .default_width(760)
            .heading(runtime.state().heading())
            .body(ui_text::TRUST_ENTRIES_DESCRIPTION)
            .extra_child(toast_overlay.widget())
            .close_response("close")
            .build();
        dialog.add_response("close", ui_text::CLOSE);
        let on_close = on_event.clone();
        dialog.connect_response(None, move |_, _| {
            on_close.send(TrustEntriesEvent::CloseRequested)
        });
        window_holder.replace(Some(dialog.downgrade()));
        Self::render(&dialog, &content, &runtime.state(), on_event.clone());

        let content_for_changes = content.clone();
        let dialog_for_changes = dialog.downgrade();
        let on_event_for_changes = on_event.clone();
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
                let Some(profile) = profile else {
                    on_event_for_changes.send(TrustEntriesEvent::ProfileRemoved);
                    break;
                };
                runtime_for_changes.dispatch_and_render(
                    TrustEntriesEvent::EntriesChanged(TrustEntriesState::from_profile(&profile)),
                    |state| {
                        Self::render(
                            &dialog,
                            &content_for_changes,
                            state,
                            on_event_for_changes.clone(),
                        );
                    },
                );
            }
        });
        dialog.present();
    }

    fn render(
        dialog: &libadwaita::MessageDialog,
        content: &gtk4::Box,
        state: &TrustEntriesState,
        on_event: UiEventTarget<TrustEntriesEvent>,
    ) {
        dialog.set_heading(Some(&state.heading()));
        while let Some(child) = content.first_child() {
            content.remove(&child);
        }
        let ca_group = libadwaita::PreferencesGroup::builder()
            .title(ui_text::TRUST_ENTRIES_CERTIFICATE_SERVERS)
            .description(ui_text::TRUST_ENTRIES_CERTIFICATE_DESCRIPTION)
            .build();
        ca_group.add(&TrustEntryView::render(
            TrustEntryViewProps {
                kind: TrustEntryKind::CertificateAuthority,
                description: ui_text::TRUST_ENTRIES_CA_DESCRIPTION,
                value: state.value(TrustEntryKind::CertificateAuthority),
            },
            on_event.clone(),
        ));
        content.append(&ca_group);

        let direct_group = libadwaita::PreferencesGroup::builder()
            .title(ui_text::TRUST_ENTRIES_DIRECT_SERVERS)
            .description(ui_text::TRUST_ENTRIES_DIRECT_DESCRIPTION)
            .build();
        direct_group.add(&TrustEntryView::render(
            TrustEntryViewProps {
                kind: TrustEntryKind::DeviceKey,
                description: ui_text::TRUST_ENTRIES_TPM_KEY_DESCRIPTION,
                value: state.value(TrustEntryKind::DeviceKey),
            },
            on_event.clone(),
        ));
        direct_group.add(&TrustEntryView::render(
            TrustEntryViewProps {
                kind: TrustEntryKind::Passkey,
                description: ui_text::TRUST_ENTRIES_PASSKEY_DESCRIPTION,
                value: state.value(TrustEntryKind::Passkey),
            },
            on_event.clone(),
        ));
        let copy_both = copy_button(TrustEntryKind::DirectKeys, on_event);
        let combined_action = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
        combined_action.set_halign(gtk4::Align::End);
        combined_action.set_margin_top(6);
        combined_action.append(&copy_both);
        direct_group.add(&combined_action);
        content.append(&direct_group);
    }
}

fn copy_button(kind: TrustEntryKind, on_event: UiEventTarget<TrustEntriesEvent>) -> gtk4::Button {
    let copy = gtk4::Button::builder()
        .label(kind.copy_label())
        .valign(gtk4::Align::Center)
        .build();
    copy.connect_clicked(move |_| on_event.send(TrustEntriesEvent::CopyRequested(kind)));
    copy
}

use keyringd_ui_core::trust_entries_window::TrustEntryViewProps;
use std::rc::Rc;
