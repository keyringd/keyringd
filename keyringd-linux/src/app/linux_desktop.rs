use crate::system::open_ssh::LinuxOpenSshService;
use anyhow::Context;
use keyringd_core::application::ActionId;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use crate::app::StandaloneKeyOperationPrompts;
use crate::app::handler::UnixSocketHandler;
use crate::app::ui::UiEventReceiver;
use crate::app::ui::profiles_window::ProfilesWindow;
use crate::app::{APPLICATION_ID, LinuxApplication};
use gtk4::glib::variant::ToVariant;
use gtk4::prelude::{ApplicationExt, ApplicationExtManual, GtkWindowExt, WidgetExt};
use gtk4::{gio, glib};
use keyringd_ui_core::key_operation::KeyOperationCoordinator;
use keyringd_ui_core::key_operation::KeyOperationEvent;

pub(crate) struct LinuxDesktop;

impl LinuxDesktop {
    pub(crate) fn open_profiles() -> anyhow::Result<()> {
        let bus = gio::bus_get_sync(gio::BusType::Session, gio::Cancellable::NONE)
            .context("could not connect to the session bus")?;
        let platform_data = glib::VariantDict::default();
        for (variable, key) in [
            ("DESKTOP_STARTUP_ID", "desktop-startup-id"),
            ("XDG_ACTIVATION_TOKEN", "activation-token"),
        ] {
            if let Ok(value) = std::env::var(variable) {
                platform_data.insert(key, value);
            }
        }
        bus.call_sync(
            Some(APPLICATION_ID),
            "/com/keyringd/Keyringd",
            "org.freedesktop.Application",
            "Activate",
            Some(&(platform_data,).to_variant()),
            Some(glib::VariantTy::UNIT),
            gio::DBusCallFlags::NONE,
            -1,
            gio::Cancellable::NONE,
        )
        .context("could not ask the Keyringd Agent to open the Profiles window")?;
        Ok(())
    }

    pub(crate) fn run_agent(
        keyringd: Arc<LinuxApplication>,
        prompts: Arc<KeyOperationCoordinator>,
        events: UiEventReceiver<(ActionId, KeyOperationEvent)>,
        socket: UnixSocketHandler,
    ) -> anyhow::Result<()> {
        libadwaita::init()?;
        let application = libadwaita::Application::builder()
            .application_id(APPLICATION_ID)
            .flags(gio::ApplicationFlags::IS_SERVICE)
            .build();
        application.set_inactivity_timeout(60_000);
        let keyringd = Rc::new(keyringd);
        let profiles = Arc::clone(keyringd.as_ref());
        let open_ssh = Arc::new(LinuxOpenSshService);
        let events = Rc::new(RefCell::new(Some(events)));
        let profiles_window = Rc::new(RefCell::new(gtk4::glib::WeakRef::<
            libadwaita::ApplicationWindow,
        >::new()));
        let prompts_for_startup = Arc::clone(&prompts);
        application.connect_startup(move |application| {
            gtk4::Window::set_default_icon_name(APPLICATION_ID);
            if let Some(events) = events.borrow_mut().take() {
                StandaloneKeyOperationPrompts::start(
                    application,
                    Arc::clone(&prompts_for_startup),
                    events,
                );
            }
        });
        let agent_coordinator = prompts.clone();
        application.connect_activate(move |application| {
            if let Some(window) = profiles_window.borrow().upgrade() {
                window.present();
                return;
            }
            let window = ProfilesWindow::present(application, &profiles, &open_ssh, &prompts);
            profiles_window.borrow().set(Some(&window));
            let profiles_window_for_close = Rc::clone(&profiles_window);
            window.connect_close_request(move |_| {
                tracing::debug!("Profiles window close requested; clearing window registry");
                profiles_window_for_close.borrow().set(None);
                gtk4::glib::Propagation::Proceed
            });
            let profiles_window_for_destroy = Rc::clone(&profiles_window);
            window.connect_destroy(move |_| {
                tracing::debug!("Profiles window destroyed; clearing window registry");
                profiles_window_for_destroy.borrow().set(None);
            });
        });

        let socket_application = Arc::clone(keyringd.as_ref());
        let (agent_activity, agent_activity_receiver) = tokio::sync::mpsc::unbounded_channel();
        tokio::spawn(async move {
            if let Err(error) = socket
                .run(socket_application, agent_coordinator, Some(agent_activity))
                .await
            {
                keyringd_core::util::FailureDiagnostics::record_error(
                    "SSH Agent listener task",
                    &error,
                );
            }
        });
        let mut agent_activity_receiver = UiEventReceiver::from_tokio(agent_activity_receiver);
        let application_for_activity = application.clone();
        gtk4::glib::MainContext::ref_thread_default().spawn_local(async move {
            let mut connection_hold = None;
            while let Some(active) = agent_activity_receiver.receive().await {
                if active && connection_hold.is_none() {
                    connection_hold = Some(application_for_activity.hold());
                } else if !active {
                    connection_hold = None;
                }
            }
        });
        let status = application.run_with_args(&["keyringd"]);
        anyhow::ensure!(
            status == gtk4::glib::ExitCode::SUCCESS,
            "Desktop agent exited with status {}",
            status.get()
        );
        Ok(())
    }
}
