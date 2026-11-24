use crate::app::ui::ssh_connection_test_window::SshConnectionTestCommandHandler;
use keyringd_core::model::ProfileSshIdentity;
use keyringd_ui_core::ssh_connection_test_window::SshConnectionTestEvent;
use keyringd_ui_core::ssh_connection_test_window::SshConnectionTestProps;
use keyringd_ui_core::ssh_connection_test_window::SshConnectionTestState;
use keyringd_ui_core::toast::ToastMessage;

use keyringd_ui_core::text as ui_text;

use gtk4::glib::object::ObjectExt;
use gtk4::prelude::{
    AccessibleExtManual, BoxExt, ButtonExt, CheckButtonExt, EditableExt, EntryExt, GtkWindowExt,
    WidgetExt,
};
use keyringd_core::model::ProfileSshIdentityKind;
use libadwaita::prelude::{ActionRowExt, MessageDialogExt, PreferencesGroupExt};

use crate::system::open_ssh::LinuxOpenSshService;

use crate::app::ui::ComponentRuntime;
use crate::app::ui::radio_button::RadioButtonView;

/// Tests one profile identity through the installed OpenSSH client.
pub(crate) struct SshConnectionTestWindow;

impl SshConnectionTestWindow {
    pub(crate) fn present(
        parent: &gtk4::Window,
        props: SshConnectionTestProps,
        open_ssh: std::sync::Arc<LinuxOpenSshService>,
        application: std::sync::Arc<crate::app::LinuxApplication>,
    ) {
        let profile = props.profile;
        let identities =
            ProfileSshIdentity::available_kinds_at(&profile, std::time::SystemTime::now());
        if identities.is_empty() {
            return;
        }
        let initial_state = SshConnectionTestState::new(identities.clone());

        let host = gtk4::Entry::builder()
            .placeholder_text(ui_text::SSH_CONNECTION_TEST_HOST_PLACEHOLDER)
            .activates_default(true)
            .build();
        host.update_property(&[gtk4::accessible::Property::Label(
            ui_text::SSH_CONNECTION_TEST_HOST_LABEL,
        )]);
        let fields = libadwaita::PreferencesGroup::builder()
            .title(ui_text::SSH_CONNECTION_TEST_HOST_LABEL)
            .build();
        fields.add(&host);
        let methods = libadwaita::PreferencesGroup::builder()
            .title(ui_text::SSH_CONNECTION_TEST_IDENTITY_LABEL)
            .build();
        let mut choices = Vec::with_capacity(identities.len());
        let mut group = None;
        for identity in identities {
            let (title, subtitle) = Self::identity_copy(&identity);
            let (choice, row) = Self::method_row(title, subtitle, group.as_ref());
            if group.is_none() {
                choice.widget().set_active(true);
                group = Some(choice.clone());
            }
            methods.add(&row);
            choices.push((choice, identity));
        }
        let choices = std::rc::Rc::new(choices);
        let test = gtk4::Button::builder()
            .label(ui_text::SSH_CONNECTION_TEST_TEST)
            .halign(gtk4::Align::End)
            .sensitive(false)
            .build();
        test.add_css_class("suggested-action");
        let runtime = std::rc::Rc::new(ComponentRuntime::new(initial_state));
        let content = gtk4::Box::new(gtk4::Orientation::Vertical, 16);
        content.append(&fields);
        content.append(&methods);
        content.append(&test);

        let dialog = crate::app::ui::message_dialog_builder()
            .transient_for(parent)
            .heading(
                keyringd_ui_core::ssh_connection_test_window::profile_heading(
                    profile.display_name(),
                ),
            )
            .body(ui_text::SSH_CONNECTION_TEST_DESCRIPTION)
            .extra_child(&content)
            .close_response("close")
            .build();
        dialog.add_response("close", ui_text::CLOSE);
        let handler = SshConnectionTestCommandHandler::new(
            open_ssh,
            application,
            profile.id().clone(),
            dialog.downgrade(),
        );
        let runtime_for_close = std::rc::Rc::clone(&runtime);
        let handler_for_close = handler.clone();
        let test_for_close = test.downgrade();
        dialog.connect_response(None, move |dialog, _| {
            let Some(test) = test_for_close.upgrade() else {
                return;
            };
            Self::dispatch(
                &runtime_for_close,
                &handler_for_close,
                dialog,
                &test,
                SshConnectionTestEvent::CloseRequested,
            );
        });
        let test_for_host = test.clone();
        let dialog_for_host = dialog.downgrade();
        let runtime_for_host = std::rc::Rc::clone(&runtime);
        host.connect_changed(move |host| {
            let Some(dialog) = dialog_for_host.upgrade() else {
                return;
            };
            runtime_for_host.dispatch_and_render(
                SshConnectionTestEvent::HostChanged(host.text().to_string()),
                |state| Self::render(&dialog, &test_for_host, state),
            );
        });
        for (choice, identity) in choices.iter() {
            let identity = *identity;
            let runtime = std::rc::Rc::clone(&runtime);
            choice.widget().connect_toggled(move |choice| {
                if choice.is_active() {
                    runtime.dispatch(SshConnectionTestEvent::IdentitySelected(identity));
                }
            });
        }
        let test_for_enter = test.clone();
        host.connect_activate(move |_| test_for_enter.emit_clicked());
        let dialog_for_test = dialog.downgrade();
        let runtime_for_test = std::rc::Rc::clone(&runtime);
        test.connect_clicked(move |test| {
            let Some(dialog) = dialog_for_test.upgrade() else {
                return;
            };
            Self::dispatch(
                &runtime_for_test,
                &handler,
                &dialog,
                test,
                SshConnectionTestEvent::TestRequested,
            );
        });
        dialog.present();
    }

    fn dispatch(
        runtime: &std::rc::Rc<ComponentRuntime<SshConnectionTestState>>,
        handler: &SshConnectionTestCommandHandler,
        dialog: &libadwaita::MessageDialog,
        test: &gtk4::Button,
        event: SshConnectionTestEvent,
    ) -> Option<ToastMessage> {
        let command =
            runtime.dispatch_and_render(event, |state| Self::render(dialog, test, state))?;
        let runtime = std::rc::Rc::clone(runtime);
        let next_handler = handler.clone();
        let dialog = dialog.downgrade();
        let test = test.downgrade();
        handler.handle(
            command,
            std::rc::Rc::new(move |event| {
                let dialog = dialog.upgrade()?;
                let test = test.upgrade()?;
                Self::dispatch(&runtime, &next_handler, &dialog, &test, event)
            }),
        )
    }

    fn method_row(
        title: &str,
        subtitle: &str,
        group: Option<&RadioButtonView>,
    ) -> (RadioButtonView, libadwaita::ActionRow) {
        let choice = RadioButtonView::new(title);
        choice
            .widget()
            .set_group(group.map(RadioButtonView::widget));
        let row = libadwaita::ActionRow::builder()
            .title(title)
            .subtitle(subtitle)
            .activatable_widget(choice.widget())
            .build();
        row.add_prefix(choice.widget());
        (choice, row)
    }

    fn identity_copy(identity: &ProfileSshIdentityKind) -> (&'static str, &'static str) {
        (
            keyringd_ui_core::ssh_connection_test_window::identity_title(*identity),
            keyringd_ui_core::ssh_connection_test_window::identity_description(
                *identity,
                ui_text::SSH_CONNECTION_TEST_TPM_KEY_DESCRIPTION,
            ),
        )
    }

    fn render(
        dialog: &libadwaita::MessageDialog,
        test: &gtk4::Button,
        state: &SshConnectionTestState,
    ) {
        if let Some(content) = dialog.extra_child() {
            content.set_sensitive(state.can_edit());
        }
        test.set_sensitive(state.can_test());
        dialog.set_response_enabled("close", !state.is_testing());
        dialog.set_body(if state.is_testing() {
            ui_text::SSH_CONNECTION_TEST_TESTING
        } else {
            state
                .error()
                .unwrap_or(ui_text::SSH_CONNECTION_TEST_DESCRIPTION)
        });
    }
}
