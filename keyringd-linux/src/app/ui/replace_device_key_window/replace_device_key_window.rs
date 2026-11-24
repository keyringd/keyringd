use super::ReplaceDeviceKeyCommandHandler;
use crate::app::LinuxApplication;
use gtk4::prelude::GtkWindowExt;
use keyringd_ui_core::ComponentModel;
use keyringd_ui_core::key_operation::KeyOperationCoordinator;
use keyringd_ui_core::replace_device_key_window::{
    ReplaceDeviceKeyEvent, ReplaceDeviceKeyProps, ReplaceDeviceKeyState,
};
use libadwaita::prelude::MessageDialogExt;
use std::{cell::RefCell, rc::Rc, sync::Arc};

pub(crate) struct ReplaceDeviceKeyWindow;

impl ReplaceDeviceKeyWindow {
    pub(crate) fn present(
        parent: &gtk4::Window,
        props: ReplaceDeviceKeyProps,
        application: libadwaita::Application,
        profile_application: Arc<LinuxApplication>,
        prompts: Arc<KeyOperationCoordinator>,
    ) {
        let handler = ReplaceDeviceKeyCommandHandler::new(
            parent.clone(),
            props,
            application,
            profile_application,
            prompts,
        );
        Self::present_state(
            parent,
            handler,
            Rc::new(RefCell::new(ReplaceDeviceKeyState::default())),
        );
    }

    fn present_state(
        parent: &gtk4::Window,
        handler: ReplaceDeviceKeyCommandHandler,
        state: Rc<RefCell<ReplaceDeviceKeyState>>,
    ) {
        let dialog = crate::app::ui::message_dialog_builder()
            .transient_for(parent)
            .heading(state.borrow().heading())
            .body(state.borrow().message())
            .close_response("close")
            .default_response("close")
            .build();
        dialog.add_response("close", state.borrow().close_label());
        dialog.add_response("primary", state.borrow().primary_label());
        dialog.set_response_appearance(
            "primary",
            if state.borrow().is_complete() {
                libadwaita::ResponseAppearance::Suggested
            } else {
                libadwaita::ResponseAppearance::Destructive
            },
        );
        let parent = parent.clone();
        dialog.connect_response(None, move |dialog, response| {
            let event = if response == "primary" {
                ReplaceDeviceKeyEvent::PrimaryRequested
            } else {
                ReplaceDeviceKeyEvent::CloseRequested
            };
            let command = state.borrow_mut().update(event);
            if let Some(command) = command {
                let parent = parent.clone();
                let next_handler = handler.clone();
                let state = Rc::clone(&state);
                handler.handle(
                    command,
                    dialog,
                    Rc::new(move |event| {
                        state.borrow_mut().update(event);
                        Self::present_state(&parent, next_handler.clone(), Rc::clone(&state));
                    }),
                );
            }
        });
        dialog.present();
    }
}
