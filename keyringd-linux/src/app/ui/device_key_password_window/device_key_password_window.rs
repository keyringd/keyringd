use keyringd_ui_core::text as ui_text;
use std::cell::RefCell;

use gtk4::glib::object::ObjectExt;
use gtk4::prelude::{EditableExt, GtkWindowExt, IsA};
use keyringd_core::service::DeviceKeyProtectionChange;
use libadwaita::prelude::{MessageDialogExt, PreferencesGroupExt};

/// Presents Device Key password prompts.
pub(crate) struct DeviceKeyPasswordWindow;

impl DeviceKeyPasswordWindow {
    pub(crate) fn request_password(
        application: &libadwaita::Application,
        parent: Option<&gtk4::Window>,
        profile_name: &str,
        confirm: bool,
        response: tokio::sync::oneshot::Sender<Option<String>>,
        events: keyringd_ui_core::runtime::UiEventSender<
            keyringd_ui_core::key_operation::KeyOperationEvent,
        >,
    ) -> libadwaita::MessageDialog {
        let reason = if confirm {
            format!("Create a password for the Device Key used by “{profile_name}”.")
        } else {
            format!("Enter the Device Key password for “{profile_name}”.")
        };
        Self::request_secret(
            application,
            parent,
            if confirm {
                "Protect Device Key"
            } else {
                "Unlock Device Key"
            },
            &reason,
            confirm,
            response,
            events,
        )
    }

    pub(crate) fn request_password_change(
        application: &libadwaita::Application,
        parent: Option<&gtk4::Window>,
        profile_name: &str,
        change: DeviceKeyProtectionChange,
        response: tokio::sync::oneshot::Sender<
            Option<keyringd_ui_core::key_operation::DeviceKeyPasswordChangeInput>,
        >,
        events: keyringd_ui_core::runtime::UiEventSender<
            keyringd_ui_core::key_operation::KeyOperationEvent,
        >,
    ) -> libadwaita::MessageDialog {
        let disabling = change == DeviceKeyProtectionChange::Disable;
        let current = (change != DeviceKeyProtectionChange::Enable).then(Self::password_row);
        let new = (change != DeviceKeyProtectionChange::Disable).then(Self::new_password_row);
        let confirmation = new.as_ref().map(|_| Self::confirmation_row());
        let fields = libadwaita::PreferencesGroup::new();
        for entry in [&current, &new, &confirmation].into_iter().flatten() {
            fields.add(entry);
        }
        let dialog = crate::app::ui::message_dialog_builder()
            .heading(match change {
                DeviceKeyProtectionChange::ChangePassword => "Change Device Key Password",
                DeviceKeyProtectionChange::Disable => "Disable Password Protection",
                DeviceKeyProtectionChange::Enable => "Protect Device Key",
            })
            .body(format!("Update password protection for “{profile_name}”."))
            .extra_child(&fields)
            .close_response("cancel")
            .default_response("apply")
            .build();
        Self::attach(&dialog, application, parent);
        dialog.add_response("cancel", ui_text::CANCEL);
        dialog.add_response("apply", if disabling { "Disable" } else { ui_text::SAVE });
        dialog.set_response_appearance(
            "apply",
            if disabling {
                libadwaita::ResponseAppearance::Destructive
            } else {
                libadwaita::ResponseAppearance::Suggested
            },
        );
        dialog.set_response_enabled("apply", false);

        let validate = {
            let dialog = dialog.downgrade();
            let fields = fields.downgrade();
            let current = current
                .as_ref()
                .map(ObjectExt::downgrade)
                .unwrap_or_default();
            let new = new.as_ref().map(ObjectExt::downgrade).unwrap_or_default();
            let confirmation = confirmation
                .as_ref()
                .map(ObjectExt::downgrade)
                .unwrap_or_default();
            move |_: &libadwaita::PasswordEntryRow| {
                let (Some(dialog), Some(fields)) = (dialog.upgrade(), fields.upgrade()) else {
                    return;
                };
                let current = current.upgrade();
                let new = new.upgrade();
                let confirmation = confirmation.upgrade();
                let complete = current.as_ref().is_none_or(|v| !v.text().is_empty())
                    && new.as_ref().is_none_or(|v| !v.text().is_empty());
                let matches = new
                    .as_ref()
                    .zip(confirmation.as_ref())
                    .is_none_or(|(a, b)| a.text() == b.text());
                dialog.set_response_enabled("apply", complete && matches);
                fields.set_description(
                    (!matches
                        && confirmation
                            .as_ref()
                            .is_some_and(|value| !value.text().is_empty()))
                    .then_some("Passwords do not match."),
                );
            }
        };
        for entry in [&current, &new, &confirmation].into_iter().flatten() {
            entry.connect_changed(validate.clone());
        }

        let response = RefCell::new(Some(response));
        dialog.connect_response(None, move |_, selected| {
            let result = (selected == "apply").then(|| match change {
                DeviceKeyProtectionChange::ChangePassword => {
                    keyringd_ui_core::key_operation::DeviceKeyPasswordChangeInput::Change {
                        current_password: current.as_ref().unwrap().text().to_string(),
                        new_password: new.as_ref().unwrap().text().to_string(),
                    }
                }
                DeviceKeyProtectionChange::Disable => {
                    keyringd_ui_core::key_operation::DeviceKeyPasswordChangeInput::Disable {
                        current_password: current.as_ref().unwrap().text().to_string(),
                    }
                }
                DeviceKeyProtectionChange::Enable => {
                    keyringd_ui_core::key_operation::DeviceKeyPasswordChangeInput::Enable {
                        new_password: new.as_ref().unwrap().text().to_string(),
                    }
                }
            });
            if let Some(response) = response.borrow_mut().take() {
                events.send(
                    keyringd_ui_core::key_operation::KeyOperationEvent::DeviceKeyPasswordChangeAnswered {
                            value: result,
                            response,
                        },
                );
            }
        });
        dialog.present();
        dialog
    }

    fn request_secret(
        application: &libadwaita::Application,
        parent: Option<&gtk4::Window>,
        title: &str,
        reason: &str,
        confirm: bool,
        response: tokio::sync::oneshot::Sender<Option<String>>,
        events: keyringd_ui_core::runtime::UiEventSender<
            keyringd_ui_core::key_operation::KeyOperationEvent,
        >,
    ) -> libadwaita::MessageDialog {
        let secret = libadwaita::PasswordEntryRow::builder()
            .title("Password")
            .activates_default(true)
            .build();
        let confirmation = confirm.then(|| {
            libadwaita::PasswordEntryRow::builder()
                .title("Confirm Password")
                .activates_default(true)
                .build()
        });
        let fields = libadwaita::PreferencesGroup::new();
        fields.add(&secret);
        if let Some(entry) = &confirmation {
            fields.add(entry);
        }
        let dialog = crate::app::ui::message_dialog_builder()
            .heading(title)
            .body(reason)
            .extra_child(&fields)
            .close_response("cancel")
            .default_response("continue")
            .build();
        Self::attach(&dialog, application, parent);
        dialog.add_response("cancel", ui_text::CANCEL);
        dialog.add_response("continue", ui_text::CONTINUE);
        dialog.set_response_appearance("continue", libadwaita::ResponseAppearance::Suggested);
        dialog.set_response_enabled("continue", false);
        let validate = {
            let dialog = dialog.downgrade();
            let secret = secret.downgrade();
            let confirmation = confirmation
                .as_ref()
                .map(ObjectExt::downgrade)
                .unwrap_or_default();
            move |_: &libadwaita::PasswordEntryRow| {
                if let (Some(dialog), Some(secret)) = (dialog.upgrade(), secret.upgrade()) {
                    let password = secret.text();
                    dialog.set_response_enabled(
                        "continue",
                        !password.is_empty()
                            && confirmation.upgrade().is_none_or(|v| v.text() == password),
                    );
                }
            }
        };
        if let Some(entry) = &confirmation {
            entry.connect_changed(validate.clone());
        }
        secret.connect_changed(validate);
        let response = RefCell::new(Some(response));
        dialog.connect_response(None, move |_, selected| {
            if let Some(response) = response.borrow_mut().take() {
                let value = (selected == "continue").then(|| secret.text().to_string());
                events.send(
                    keyringd_ui_core::key_operation::KeyOperationEvent::DeviceKeyPasswordAnswered {
                        value,
                        response,
                    },
                );
            }
        });
        dialog.present();
        dialog
    }

    fn password_row() -> libadwaita::PasswordEntryRow {
        libadwaita::PasswordEntryRow::builder()
            .title("Current password")
            .activates_default(true)
            .build()
    }

    fn new_password_row() -> libadwaita::PasswordEntryRow {
        libadwaita::PasswordEntryRow::builder()
            .title("New password")
            .activates_default(true)
            .build()
    }

    fn confirmation_row() -> libadwaita::PasswordEntryRow {
        libadwaita::PasswordEntryRow::builder()
            .title("Confirm new password")
            .activates_default(true)
            .build()
    }

    fn attach(
        window: &impl IsA<gtk4::Window>,
        application: &libadwaita::Application,
        parent: Option<&gtk4::Window>,
    ) {
        window.set_application(Some(application));
        if let Some(parent) = parent {
            window.set_transient_for(Some(parent));
        }
    }
}
