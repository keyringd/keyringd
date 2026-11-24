use keyringd_ui_core::runtime::UiEventTarget;
use std::sync::Arc;

use crate::app::LinuxApplication;
use crate::app::ui::ComponentRuntime;
use gtk4::prelude::GtkWindowExt;
use keyringd_ui_core::device_key_protection_window::DeviceKeyProtectionResult;
use keyringd_ui_core::key_operation::KeyOperationCoordinator;
use keyringd_ui_core::text as ui_text;
use libadwaita::prelude::MessageDialogExt;

use crate::app::ui::device_key_protection_window::DeviceKeyProtectionCommandHandler;
use keyringd_ui_core::device_key_protection_window::DeviceKeyProtectionEvent;
use keyringd_ui_core::device_key_protection_window::DeviceKeyProtectionProps;
use keyringd_ui_core::device_key_protection_window::DeviceKeyProtectionState;

pub(crate) struct DeviceKeyProtectionWindow;

impl DeviceKeyProtectionWindow {
    pub(crate) fn present(
        parent: &gtk4::Window,
        props: DeviceKeyProtectionProps,
        application: libadwaita::Application,
        profile_application: Arc<LinuxApplication>,
        prompts: Arc<KeyOperationCoordinator>,
        on_completed: UiEventTarget<DeviceKeyProtectionResult>,
    ) {
        let state = ComponentRuntime::new(DeviceKeyProtectionState::ready(props.change));
        let handler = DeviceKeyProtectionCommandHandler::new(
            parent.clone(),
            props,
            application,
            profile_application,
            prompts,
            on_completed,
        );
        gtk4::glib::MainContext::ref_thread_default().spawn_local(async move {
            let mut event = DeviceKeyProtectionEvent::Started;
            while let Some(command) = state.dispatch(event) {
                let Some(next) = handler.handle(command).await else {
                    break;
                };
                event = next;
            }
        });
    }

    pub(super) async fn confirm_retry(parent: &gtk4::Window, error: &str) -> bool {
        let dialog = crate::app::ui::message_dialog_builder()
            .transient_for(parent)
            .heading(ui_text::DEVICE_KEY_PROTECTION_ERROR_TITLE)
            .body(error)
            .close_response("cancel")
            .default_response("retry")
            .build();
        dialog.add_response("cancel", ui_text::CANCEL);
        dialog.add_response("retry", ui_text::DEVICE_KEY_PROTECTION_RETRY);
        let (response, receiver) = tokio::sync::oneshot::channel();
        let response = std::cell::RefCell::new(Some(response));
        dialog.connect_response(None, move |dialog, selected| {
            if let Some(response) = response.borrow_mut().take() {
                let _ = response.send(selected == "retry");
            }
            dialog.close();
        });
        dialog.present();
        receiver.await.unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gtk4::prelude::Cast;

    keyringd_testkit::scenario! {
        group "DeviceKeyProtectionWindow::confirm_retry" {
            case "Retry dialog reports the selected choice" {
                verify "return the retry choice corresponding to the button pressed" {
                    gtk4::test_synced(|| {
                        libadwaita::init().unwrap();
                        let parent = gtk4::Window::new();
                        parent.present();
                        for (action, expected) in [("retry", true), ("cancel", false)] {
                            gtk4::glib::idle_add_local_once(move || {
                                let dialog = gtk4::Window::list_toplevels()
                                    .into_iter()
                                    .filter_map(|window| window.downcast::<libadwaita::MessageDialog>().ok())
                                    .find(|dialog| {
                                        dialog.heading().as_deref()
                                            == Some(ui_text::DEVICE_KEY_PROTECTION_ERROR_TITLE)
                                    })
                                    .expect("the retry choice is presented");
                                dialog.response(action);
                            });
                            assert_eq!(
                                gtk4::glib::MainContext::ref_thread_default().block_on(
                                    DeviceKeyProtectionWindow::confirm_retry(&parent, "retryable error")
                                ),
                                expected
                            );
                        }
                        parent.close();
                    });
                }
            }
        }
    }
}
