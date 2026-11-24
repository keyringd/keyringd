use super::ProfileSetupWindowCommandHandler;
use crate::app::LinuxApplication;
use crate::app::ui::profile_setup_wizard::ProfileSetupWizardView;
use crate::app::ui::{ComponentRuntime, PASSKEY_DIALOG_WIDTH};
use gtk4::prelude::{EventControllerExt, GtkWindowExt, WidgetExt};
use keyringd_ui_core::key_operation::KeyOperationCoordinator;

use keyringd_ui_core::profile_setup_window::ProfileSetupEvent;
use keyringd_ui_core::profile_setup_window::{ProfileSetupWindowEvent, ProfileSetupWindowState};

use keyringd_ui_core::runtime::UiEventTarget;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

pub(crate) struct ProfileSetupWindow {
    dialog: libadwaita::Window,
    view: ProfileSetupWizardView,
    runtime: ComponentRuntime<ProfileSetupWindowState>,
    handler: Rc<ProfileSetupWindowCommandHandler>,
}

impl ProfileSetupWindow {
    pub(crate) fn present(
        parent: &libadwaita::ApplicationWindow,
        application: libadwaita::Application,
        profile_application: Arc<LinuxApplication>,
        prompts: Arc<KeyOperationCoordinator>,
    ) {
        let window = Rc::new_cyclic(|weak: &std::rc::Weak<Self>| {
            let weak = weak.clone();
            let view = ProfileSetupWizardView::new(UiEventTarget::new(move |event| {
                if let Some(window) = weak.upgrade() {
                    window.handle_event(event);
                }
            }));
            let dialog = libadwaita::Window::builder()
                .transient_for(parent)
                .modal(true)
                .default_width(PASSKEY_DIALOG_WIDTH)
                .default_height(440)
                .content(view.widget())
                .accessible_role(gtk4::AccessibleRole::Dialog)
                .build();
            let handler = ProfileSetupWindowCommandHandler::new(
                dialog.clone(),
                application,
                profile_application,
                prompts,
            );
            Self {
                dialog,
                view,
                runtime: ComponentRuntime::new(ProfileSetupWindowState::default()),
                handler,
            }
        });
        let weak = Rc::downgrade(&window);
        window.dialog.connect_close_request(move |_| {
            if let Some(window) = weak.upgrade() {
                window.handle_event(ProfileSetupEvent::Dismissed);
            }
            gtk4::glib::Propagation::Stop
        });
        let escape = gtk4::EventControllerKey::new();
        escape.set_propagation_phase(gtk4::PropagationPhase::Capture);
        let weak = Rc::downgrade(&window);
        escape.connect_key_pressed(move |_, key, _, _| {
            if key != gtk4::gdk::Key::Escape {
                return gtk4::glib::Propagation::Proceed;
            }
            if let Some(window) = weak.upgrade() {
                window.handle_event(ProfileSetupEvent::Dismissed);
            }
            gtk4::glib::Propagation::Stop
        });
        window.dialog.add_controller(escape);
        let (closed, wait_closed) = tokio::sync::oneshot::channel();
        let closed = RefCell::new(Some(closed));
        window.dialog.connect_destroy(move |_| {
            if let Some(closed) = closed.borrow_mut().take() {
                let _ = closed.send(());
            }
        });
        window.render();
        window.dialog.present();
        // The native window owns this lifetime; View event connections stay weak.
        gtk4::glib::MainContext::ref_thread_default().spawn_local(async move {
            let _ = wait_closed.await;
            window.handler.cancel();
        });
    }

    fn render(&self) {
        let state = self.runtime.state();
        self.dialog.set_deletable(!state.is_running());
        self.view.render(&state);
    }

    fn handle_event(self: &Rc<Self>, event: impl Into<ProfileSetupWindowEvent>) {
        let span = self.handler.log_span();
        let _entered = span.as_ref().map(tracing::Span::enter);
        let command = self.runtime.dispatch(event.into());
        self.render();
        if let Some(command) = command {
            let weak = Rc::downgrade(self);
            self.handler.handle(
                command,
                UiEventTarget::new(move |event| {
                    if let Some(window) = weak.upgrade() {
                        window.handle_event(event);
                    }
                }),
            );
        }
    }
}
