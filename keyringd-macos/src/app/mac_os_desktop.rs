use super::StandaloneKeyOperationPrompts;
use crate::app::ui::MainThreadEventReceiverExt as _;
use keyringd_core::application::ActionId;
use std::cell::RefCell;
use std::sync::Arc;

use dispatch2::{DispatchQueue, MainThreadBound};
use keyringd_ui_core::runtime::{UiEventSender, UiEventTarget};
use objc2::MainThreadMarker;
use objc2_app_kit::{NSAlert, NSApplication};
use objc2_foundation::NSString;

use super::handler::UnixSocketHandler;
use super::ui::UiEventReceiver;
use super::ui::menu_bar::MenuBarComponent;
use super::ui::profile_setup_window::ProfileSetupWindow;
use crate::app::MacOsApplication;
use crate::app::ui::AppShell;
use crate::app::ui::profile_list_item::ProfileListItemEvent;
use crate::app::ui::profiles_window::ProfilesWindow;
use crate::app::ui::profiles_window::ProfilesWindowCommandHandler;
use crate::app::ui::profiles_window::ProfilesWindowEvent;
use crate::app::ui::ssh_client_configuration_window::SshClientConfigurationEvent;
use crate::app::ui::ssh_client_configuration_window::SshClientConfigurationWindow;
use keyringd_ui_core::app_menu::{AppMenuEvent, AppMenuLifecycleEvent};
use keyringd_ui_core::key_operation::{KeyOperationCoordinator, KeyOperationEvent};

pub(crate) struct MacOsDesktop {
    runtime: tokio::runtime::Runtime,
    interaction: Arc<KeyOperationCoordinator>,
    keyringd: Arc<MacOsApplication>,
    events: UiEventReceiver<(ActionId, KeyOperationEvent)>,
    socket: UnixSocketHandler,
    home: std::path::PathBuf,
    settings: Arc<keyringd_core::application::SettingStore>,
}

impl MacOsDesktop {
    pub(crate) fn new(
        runtime: tokio::runtime::Runtime,
        interaction: Arc<KeyOperationCoordinator>,
        keyringd: Arc<MacOsApplication>,
        events: UiEventReceiver<(ActionId, KeyOperationEvent)>,
        socket: UnixSocketHandler,
        home: std::path::PathBuf,
        settings: Arc<keyringd_core::application::SettingStore>,
    ) -> Self {
        Self {
            runtime,
            interaction,
            keyringd,
            events,
            socket,
            home,
            settings,
        }
    }

    pub(crate) fn run(self) -> anyhow::Result<()> {
        let mtm = MainThreadMarker::new()
            .ok_or_else(|| anyhow::anyhow!("AppKit must run on the main thread"))?;
        let application = NSApplication::sharedApplication(mtm);
        let (initial_profiles, mut profile_changes) = UiEventReceiver::from_tokio_watch(
            self.runtime.handle(),
            self.keyringd.subscribe_profile_changes(),
        );
        let socket_path = self.socket.path().display().to_string();
        let (menu_component, menu_events) = MenuBarComponent::mount(initial_profiles.clone(), mtm)?;
        let configure_ssh = UiEventTarget::from_sender(menu_component.events()).map(|()| {
            AppMenuLifecycleEvent::ActionRequested(AppMenuEvent::ShowSshClientConfiguration)
        });
        let (profiles_component, mut profile_events) =
            ProfilesWindow::new(initial_profiles, configure_ssh, mtm);
        let profiles_window = Arc::new(MainThreadBound::new(RefCell::new(profiles_component), mtm));
        let shell = Arc::new(MainThreadBound::new(
            AppShell::install(mtm, &self.home, menu_component.events(), self.socket.path()),
            mtm,
        ));
        let setup_window = ProfileSetupWindow::new(
            self.runtime.handle().clone(),
            Arc::clone(&self.keyringd),
            Arc::clone(&self.interaction),
            mtm,
        );
        let menu_bar = Arc::new(MainThreadBound::new(RefCell::new(menu_component), mtm));
        let selection = Arc::new(super::SshAgentSelectionService::new(
            Arc::clone(&self.settings),
            crate::system::open_ssh::MacOsSshAgentConfiguration::new(
                self.home.clone(),
                self.socket.path().to_path_buf(),
            ),
        ));
        let (ssh_client_configuration_window, ssh_client_configuration_events) =
            SshClientConfigurationWindow::new(
                Arc::clone(&selection),
                self.runtime.handle().clone(),
                mtm,
            );
        let ssh_client_configuration =
            Arc::new(MainThreadBound::new(ssh_client_configuration_window, mtm));
        {
            let configuration = selection;
            self.runtime.spawn(async move {
                loop {
                    match configuration.reconcile() {
                        Ok(()) => break,
                        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                        }
                        Err(error) => {
                            tracing::error!(%error, "could not apply the saved SSH agent selection");
                            DispatchQueue::main().exec_async(move || {
                                let mtm = MainThreadMarker::new().expect("AppKit main queue");
                                let alert = NSAlert::new(mtm);
                                alert.setMessageText(&NSString::from_str("SSH client configuration could not be applied"));
                                alert.setInformativeText(&NSString::from_str(&error.to_string()));
                                alert.runModal();
                            });
                            break;
                        }
                    }
                }
            });
        }
        let ssh_client_configuration_for_events = Arc::clone(&ssh_client_configuration);
        ssh_client_configuration_events.spawn_on_main(self.runtime.handle(), move |event, mtm| {
            ssh_client_configuration_for_events
                .get(mtm)
                .dispatch(event, mtm);
        });
        let (profiles_events, profiles_event_receiver) = UiEventSender::channel();
        let profiles_parent = profiles_window.get(mtm).borrow().window_handle();
        let profiles_command_handler = Arc::new(MainThreadBound::new(
            RefCell::new(ProfilesWindowCommandHandler::new(
                Arc::clone(&self.keyringd),
                self.runtime.handle().clone(),
                socket_path.clone(),
                self.home.clone(),
                profiles_parent,
                Arc::clone(&setup_window),
                shell.get(mtm).clone(),
                profiles_events.clone(),
                Arc::clone(&self.interaction),
                mtm,
            )),
            mtm,
        ));
        let profiles_command_handler_for_events = Arc::clone(&profiles_command_handler);
        let profiles_window_for_events = Arc::clone(&profiles_window);
        let application_for_profile_events = Arc::clone(&self.keyringd);
        profiles_event_receiver.spawn_on_main(self.runtime.handle(), move |event, mtm| {
            let event = match event {
                ProfilesWindowEvent::ProfilesChanged(_) => {
                    ProfilesWindowEvent::ProfilesChanged(application_for_profile_events.profiles())
                }
                event => event,
            };
            if matches!(event, ProfilesWindowEvent::MutationFinished { .. }) {
                profiles_window_for_events.get(mtm).borrow_mut().dispatch(
                    ProfilesWindowEvent::ProfilesChanged(application_for_profile_events.profiles()),
                    mtm,
                );
            }
            if let Some(command) = profiles_window_for_events
                .get(mtm)
                .borrow_mut()
                .dispatch(event, mtm)
            {
                profiles_command_handler_for_events
                    .get(mtm)
                    .borrow_mut()
                    .handle(command, mtm);
            }
        });

        let (standalone_prompts, prompt_inputs) = StandaloneKeyOperationPrompts::new(
            mtm,
            Arc::clone(&self.interaction),
            shell.get(mtm).clone(),
        );
        let prompts = Arc::new(MainThreadBound::new(RefCell::new(standalone_prompts), mtm));
        let prompts_for_inputs = Arc::downgrade(&prompts);
        prompt_inputs.spawn_on_main(self.runtime.handle(), move |(flow_id, event), mtm| {
            let Some(prompts_for_inputs) = prompts_for_inputs.upgrade() else {
                event.reject();
                return;
            };
            prompts_for_inputs
                .get(mtm)
                .borrow_mut()
                .dispatch_input(flow_id, event);
        });
        let prompts_for_events = Arc::clone(&prompts);
        self.events
            .spawn_on_main(self.runtime.handle(), move |(flow_id, event), mtm| {
                prompts_for_events
                    .get(mtm)
                    .borrow_mut()
                    .dispatch(flow_id, event);
            });
        let menu_for_signal = Arc::clone(&menu_bar);
        self.runtime.spawn(async move {
            let mut terminate =
                match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
                    Ok(signal) => signal,
                    Err(error) => {
                        tracing::error!(%error, "could not install SIGTERM handler");
                        return;
                    }
                };
            let mut interrupt =
                match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt()) {
                    Ok(signal) => signal,
                    Err(error) => {
                        tracing::error!(%error, "could not install SIGINT handler");
                        return;
                    }
                };
            tokio::select! {
                _ = terminate.recv() => {}
                _ = interrupt.recv() => {}
            }
            DispatchQueue::main().exec_async(move || {
                let mtm = MainThreadMarker::new()
                    .expect("the callback should run on the AppKit thread because it was dispatched to the GCD main queue");
                menu_for_signal.get(mtm).borrow_mut().stop(mtm);
                NSApplication::sharedApplication(mtm).terminate(None);
            });
        });
        let profiles_for_menu = Arc::clone(&profiles_window);
        let profiles_events_for_menu = profiles_events.clone();
        let ssh_client_configuration_for_menu = Arc::clone(&ssh_client_configuration);
        let shell_for_menu = Arc::clone(&shell);
        let menu_for_actions = Arc::clone(&menu_bar);
        menu_events.spawn_on_main(self.runtime.handle(), move |event, mtm| {
            let Some(event) = menu_for_actions.get(mtm).borrow_mut().dispatch(event, mtm) else {
                return;
            };
            match event {
                event @ (AppMenuEvent::OpenProfiles | AppMenuEvent::AddProfile) => {
                    let window = profiles_for_menu.get(mtm).borrow().window_handle();
                    if !window.isVisible() {
                        profiles_events_for_menu.send(ProfilesWindowEvent::Opened);
                    }
                    shell_for_menu.get(mtm).present(&window);
                    if matches!(event, AppMenuEvent::AddProfile) {
                        profiles_events_for_menu.send(ProfilesWindowEvent::AddRequested);
                    }
                }
                AppMenuEvent::ShowAbout => {
                    shell_for_menu.get(mtm).show_about();
                }
                AppMenuEvent::ShowSshClientConfiguration => {
                    shell_for_menu.get(mtm).activate_for_window();
                    ssh_client_configuration_for_menu
                        .get(mtm)
                        .dispatch(SshClientConfigurationEvent::OpenRequested, mtm);
                }
                AppMenuEvent::SetProfileEnabled { id, enabled } => {
                    profiles_events_for_menu.send(ProfilesWindowEvent::ProfileAction(Box::new(
                        ProfileListItemEvent::SetEnabled { id, enabled },
                    )));
                }
                AppMenuEvent::ShowProfileDetails(id) => {
                    profiles_events_for_menu.send(ProfilesWindowEvent::ProfileAction(Box::new(
                        ProfileListItemEvent::ShowDetails(id),
                    )));
                }
                AppMenuEvent::ShowTrustEntries(id) => {
                    profiles_events_for_menu.send(ProfilesWindowEvent::ProfileAction(Box::new(
                        ProfileListItemEvent::ShowTrustEntries(id),
                    )));
                }
                AppMenuEvent::TestConnection(id) => {
                    let window = profiles_for_menu.get(mtm).borrow().window_handle();
                    shell_for_menu.get(mtm).present(&window);
                    profiles_events_for_menu.send(ProfilesWindowEvent::ProfileAction(Box::new(
                        ProfileListItemEvent::TestConnection(id),
                    )));
                }
                AppMenuEvent::Quit => {
                    NSApplication::sharedApplication(mtm).terminate(None);
                }
            }
        });
        let profiles_events_for_window = profiles_events.clone();
        self.runtime.spawn(async move {
            while let Some(event) = profile_events.receive().await {
                profiles_events_for_window.send(event);
            }
        });
        let profiles_events_for_changes = profiles_events.clone();
        let menu_for_changes = Arc::clone(&menu_bar);
        self.runtime.spawn(async move {
            while let Some(values) = profile_changes.receive().await {
                let menu_values = values.clone();
                let menu = Arc::clone(&menu_for_changes);
                let profiles_events = profiles_events_for_changes.clone();
                DispatchQueue::main().exec_async(move || {
                    let mtm = MainThreadMarker::new()
                        .expect("the callback should run on the AppKit thread because it was dispatched to the GCD main queue");
                    profiles_events.send(ProfilesWindowEvent::ProfilesChanged(values));
                    menu.get(mtm).borrow_mut().dispatch(AppMenuLifecycleEvent::ProfilesChanged(menu_values), mtm);
                });
            }
            DispatchQueue::main().exec_async(move || {
                let mtm = MainThreadMarker::new()
                    .expect("the callback should run on the AppKit thread because it was dispatched to the GCD main queue");
                menu_for_changes.get(mtm).borrow_mut().stop(mtm);
                let alert = NSAlert::new(mtm);
                alert.setMessageText(&NSString::from_str("Profile updates stopped"));
                alert.setInformativeText(&NSString::from_str(
                    "The profile settings source closed unexpectedly and Keyringd must quit.",
                ));
                alert.runModal();
                NSApplication::sharedApplication(mtm).terminate(None);
            });
        });
        let menu_for_agent = Arc::clone(&menu_bar);
        let agent_coordinator = self.interaction.clone();
        let keyringd = self.keyringd;
        self.runtime.spawn(async move {
            if let Err(error) = self.socket.run(keyringd, agent_coordinator).await {
                keyringd_core::util::FailureDiagnostics::record_error("SSH Agent listener task", &error);
                DispatchQueue::main().exec_async(move || {
                    let mtm = MainThreadMarker::new()
                        .expect("the callback should run on the AppKit thread because it was dispatched to the GCD main queue");
                    menu_for_agent.get(mtm).borrow_mut().stop(mtm);
                    let alert = NSAlert::new(mtm);
                    alert.setMessageText(&NSString::from_str("SSH Agent stopped"));
                    alert.setInformativeText(&NSString::from_str(
                        "The SSH Agent stopped and Keyringd must quit. Restart the app before trying to connect again.",
                    ));
                    alert.runModal();
                    NSApplication::sharedApplication(mtm).terminate(None);
                });
            }
        });
        let menu_for_shutdown = Arc::clone(&menu_bar);
        let _application_lifetime = (
            self.runtime,
            self.interaction,
            prompts,
            profiles_command_handler,
            ssh_client_configuration,
            profiles_window,
            menu_bar,
            setup_window,
            shell,
        );
        application.run();
        menu_for_shutdown.get(mtm).borrow_mut().stop(mtm);
        Ok(())
    }
}
