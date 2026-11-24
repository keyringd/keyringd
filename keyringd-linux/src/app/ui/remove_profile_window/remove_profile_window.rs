use crate::app::LinuxApplication;
use crate::app::ui::remove_profile_window::RemoveProfileCommandHandler;
use gtk4::glib::object::ObjectExt;
use gtk4::prelude::GtkWindowExt;
use keyringd_ui_core::ComponentModel;
use keyringd_ui_core::remove_profile_window::RemoveProfileProps;
use keyringd_ui_core::remove_profile_window::{RemoveProfileEvent, RemoveProfileState};
use keyringd_ui_core::runtime::UiEventTarget;
use keyringd_ui_core::text as ui_text;
use keyringd_ui_core::toast::ToastMessage;
use libadwaita::prelude::MessageDialogExt;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

pub(crate) struct RemoveProfileWindow;

impl RemoveProfileWindow {
    pub(crate) fn present(
        parent: &gtk4::Window,
        props: RemoveProfileProps,
        profile_application: Arc<LinuxApplication>,
        on_toast: UiEventTarget<ToastMessage>,
    ) {
        let profile_name = props.profile_name.clone();
        let dialog = crate::app::ui::message_dialog_builder()
            .transient_for(parent)
            .heading(keyringd_ui_core::remove_profile_window::heading(
                &profile_name,
            ))
            .body(ui_text::REMOVE_PROFILE_DESCRIPTION)
            .close_response("cancel")
            .default_response("cancel")
            .build();
        dialog.add_response("cancel", ui_text::CANCEL);
        dialog.add_response("remove", ui_text::REMOVE_PROFILE_REMOVE);
        dialog.set_response_appearance("remove", libadwaita::ResponseAppearance::Destructive);
        let state = Rc::new(RefCell::new(RemoveProfileState::default()));
        let handler = RemoveProfileCommandHandler::new(
            props,
            profile_application,
            dialog.downgrade(),
            on_toast,
        );
        dialog.connect_response(None, move |dialog, response| {
            Self::dispatch(
                &state,
                &handler,
                dialog,
                &profile_name,
                if response == "remove" {
                    RemoveProfileEvent::RemovalRequested
                } else {
                    RemoveProfileEvent::CloseRequested
                },
            );
        });
        dialog.present();
    }

    fn dispatch(
        state: &Rc<RefCell<RemoveProfileState>>,
        handler: &RemoveProfileCommandHandler,
        dialog: &libadwaita::MessageDialog,
        name: &str,
        event: RemoveProfileEvent,
    ) {
        let command = state.borrow_mut().update(event);
        Self::render(dialog, &state.borrow(), name);
        if let Some(command) = command {
            let state = Rc::clone(state);
            let next_handler = handler.clone();
            let dialog = dialog.downgrade();
            let name = name.to_owned();
            handler.handle(
                command,
                Rc::new(move |event| {
                    if let Some(dialog) = dialog.upgrade() {
                        Self::dispatch(&state, &next_handler, &dialog, &name, event);
                    }
                }),
            );
        }
    }

    fn render(dialog: &libadwaita::MessageDialog, state: &RemoveProfileState, profile_name: &str) {
        dialog.set_heading(Some(&state.heading(profile_name)));
        dialog.set_response_enabled("remove", state.can_remove());
        dialog.set_response_label("remove", state.remove_label());
        dialog.set_response_label("cancel", state.close_label());
        dialog.set_body(state.message());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    keyringd_testkit::scenario! {
        group "RemoveProfileWindow::render" {
            case "Removal rendering keeps close available and enables retry after failure" {
                verify "retain the close action while rendering retry after a removal error" {
                    gtk4::test_synced(|| {
                        libadwaita::init().unwrap();
                        let dialog = crate::app::ui::message_dialog_builder()
                            .heading(keyringd_ui_core::remove_profile_window::heading("Work"))
                            .body(ui_text::REMOVE_PROFILE_DESCRIPTION)
                            .build();
                        dialog.add_response("cancel", ui_text::CANCEL);
                        dialog.add_response("remove", ui_text::REMOVE_PROFILE_REMOVE);
                        let mut state = RemoveProfileState::default();
                        state.update(RemoveProfileEvent::RemovalRequested);
                        RemoveProfileWindow::render(&dialog, &state, "Work");
                        assert!(dialog.is_response_enabled("cancel"));
                        assert!(!dialog.is_response_enabled("remove"));
                        assert_eq!(dialog.body().as_str(), ui_text::REMOVE_PROFILE_REMOVING);

                        state.update(RemoveProfileEvent::RemovalFinished(Err(
                            "storage unavailable".into(),
                        )));
                        RemoveProfileWindow::render(&dialog, &state, "Work");
                        assert!(dialog.is_response_enabled("remove"));
                        assert!(dialog.is_response_enabled("cancel"));
                        assert_eq!(dialog.body().as_str(), "storage unavailable");
                        assert_eq!(
                            dialog.heading().as_deref(),
                            Some(ui_text::REMOVE_PROFILE_ERROR_TITLE)
                        );
                    });
                }
            }
        }
    }
}
