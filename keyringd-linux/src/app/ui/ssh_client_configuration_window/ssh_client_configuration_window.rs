use crate::app::ui::ssh_client_configuration_window::SshClientConfigurationCommandHandler;
use keyringd_ui_core::ssh_client_configuration_window::SshClientConfigurationEvent;
use keyringd_ui_core::ssh_client_configuration_window::SshClientConfigurationState;

use keyringd_ui_core::text as ui_text;

use std::rc::Rc;
use std::sync::Arc;

use gtk4::prelude::{ButtonExt, GtkWindowExt, ObjectExt, WidgetExt};
use libadwaita::prelude::{ActionRowExt, MessageDialogExt, PreferencesGroupExt};

use crate::app::ui::ComponentRuntime;
use crate::system::open_ssh::LinuxOpenSshService;

pub(crate) struct SshClientConfigurationWindow;

impl SshClientConfigurationWindow {
    pub(crate) fn present(parent: &gtk4::Window, open_ssh: Arc<LinuxOpenSshService>) {
        let configure = gtk4::Button::builder()
            .label(ui_text::SSH_CLIENT_CONFIGURATION_CONFIGURE)
            .valign(gtk4::Align::Center)
            .build();
        let ssh_row = libadwaita::ActionRow::builder()
            .title("SSH_AUTH_SOCK")
            .subtitle(ui_text::SSH_CLIENT_CONFIGURATION_AGENT_DESCRIPTION)
            .build();
        ssh_row.add_suffix(&configure);

        let steps = libadwaita::PreferencesGroup::new();
        steps.add(&ssh_row);
        let dialog = crate::app::ui::message_dialog_builder()
            .transient_for(parent)
            .heading(ui_text::SSH_CLIENT_CONFIGURATION_WINDOW_TITLE)
            .body(ui_text::SSH_CLIENT_CONFIGURATION_DESCRIPTION)
            .extra_child(&steps)
            .close_response("close")
            .default_response("close")
            .build();
        dialog.add_response("close", ui_text::CLOSE);
        dialog.connect_response(None, |dialog: &libadwaita::MessageDialog, _| dialog.close());

        let handler = SshClientConfigurationCommandHandler::new(open_ssh, parent.downgrade());
        let runtime = Rc::new(ComponentRuntime::new(SshClientConfigurationState::new(
            handler.has_managed_agent_configuration(),
        )));
        Self::render(&configure, &steps, &runtime.state());
        let steps = steps.downgrade();
        configure.connect_clicked(move |configure| {
            let Some(steps) = steps.upgrade() else { return };
            Self::dispatch(
                &runtime,
                &handler,
                configure,
                &steps,
                SshClientConfigurationEvent::ConfigureRequested,
            );
        });
        dialog.present();
    }

    fn dispatch(
        runtime: &Rc<ComponentRuntime<SshClientConfigurationState>>,
        handler: &SshClientConfigurationCommandHandler,
        configure: &gtk4::Button,
        steps: &libadwaita::PreferencesGroup,
        event: SshClientConfigurationEvent,
    ) {
        let command =
            runtime.dispatch_and_render(event, |state| Self::render(configure, steps, state));
        if let Some(command) = command {
            let configured = runtime.state().configured();
            let runtime = Rc::clone(runtime);
            let next_handler = handler.clone();
            let configure = configure.downgrade();
            let steps = steps.downgrade();
            handler.handle(
                command,
                configured,
                Rc::new(move |event| {
                    if let (Some(configure), Some(steps)) = (configure.upgrade(), steps.upgrade()) {
                        Self::dispatch(&runtime, &next_handler, &configure, &steps, event);
                    }
                }),
            );
        }
    }
    fn render(
        configure: &gtk4::Button,
        steps: &libadwaita::PreferencesGroup,
        state: &SshClientConfigurationState,
    ) {
        configure.set_label(state.button_label());
        configure.set_sensitive(state.can_configure());
        steps.set_description(state.status_message());
    }
}
