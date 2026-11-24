use futures_util::stream::{FuturesUnordered, StreamExt};
use keyringd_core::application::ActionId;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::app::AuthenticationPromptCommandHandler;
use crate::app::StandaloneKeyOperationPrompts;
use crate::app::WindowsApplication;
use crate::app::handler::{GitBashHandler, NamedPipeHandler};
use crate::app::ui::about_window::AboutWindow;
use crate::app::ui::agent_prompt_window::AgentPromptWindow;
use crate::app::ui::profiles_window::ProfilesWindow;
use crate::app::ui::ssh_client_configuration_window::SshClientConfigurationWindow;
use crate::app::ui::tray::TrayHost;
use crate::app::ui::util::ReactorUiRuntime;
use keyringd_core::model::ProfileId;
use keyringd_ui_core::app_menu::AppMenuEvent;
use keyringd_ui_core::key_operation::KeyOperationCoordinator;
use keyringd_ui_core::profiles_window::ProfileMutation;

pub struct WindowsDesktop;

impl WindowsDesktop {
    pub(crate) async fn run(
        application: Arc<WindowsApplication>,
        fido_handler: Arc<KeyOperationCoordinator>,
        fido_events: keyringd_ui_core::runtime::UiEventReceiver<(
            ActionId,
            keyringd_ui_core::key_operation::KeyOperationEvent,
        )>,
        agent_prompt: Arc<AgentPromptWindow>,
        data_dir: &std::path::Path,
    ) -> anyhow::Result<()> {
        tracing::debug!("WSL clients connect through `keyringd-wsl-proxy`");
        let mut profile_changes = application.subscribe_profile_changes();
        let mut tray = TrayHost::start(application.profiles())?;
        let runtime = ReactorUiRuntime::start()?;
        let prompts = AuthenticationPromptCommandHandler::new(
            runtime.dispatcher(),
            tokio::runtime::Handle::current(),
            fido_handler.clone(),
        );
        tokio::spawn(StandaloneKeyOperationPrompts::new(prompts, fido_events).run());

        let ssh_client_configuration_window = SshClientConfigurationWindow::new(
            #[cfg(feature = "test-support")]
            None,
        );
        let profiles_window = {
            let profiles = Arc::clone(&application);
            let coordinator = Arc::clone(&fido_handler);
            let tokio_runtime = tokio::runtime::Handle::current();
            let configure_ssh = ssh_client_configuration_window.clone();
            runtime.invoke(move || {
                Ok(Arc::new(ProfilesWindow::new(
                    profiles,
                    coordinator,
                    tokio_runtime,
                    configure_ssh,
                    #[cfg(feature = "test-support")]
                    None,
                )))
            })?
        };
        let ssh_client_configuration_window_open = Arc::new(AtomicBool::new(false));
        let about_window = AboutWindow::new(runtime.dispatcher());

        if application.profiles().is_empty() {
            Self::show_profiles(&runtime, &profiles_window)?;
        }

        let device_key_context = crate::system::device_key::CngOperationContext::for_window(
            agent_prompt.hwnd().0 as usize,
        );
        let mut named_pipe = Box::pin(NamedPipeHandler::run(
            Arc::clone(&application),
            device_key_context,
            fido_handler.clone(),
        ));
        let socket_path = data_dir.join("agent.sock");
        let mut git_bash = Box::pin(GitBashHandler::run(
            &socket_path,
            Arc::clone(&application),
            device_key_context,
            fido_handler.clone(),
        ));
        let mut tray_mutations = FuturesUnordered::new();
        loop {
            tokio::select! {
                result = &mut git_bash => {
                    if let Err(error) = &result {
                        tracing::error!(%error, "Git for Windows agent stopped");
                    }
                    return result.map_err(Into::into);
                }
                result = &mut named_pipe => {
                    if let Err(error) = &result {
                        tracing::error!(%error, "Windows agent stopped");
                    }
                    return result.map_err(Into::into);
                }
                Some(result) = tray_mutations.next(), if !tray_mutations.is_empty() => {
                    if let Err(error) = result {
                        tracing::error!(%error, "could not update profile from tray");
                        tray.post_profiles_update(application.profiles())?;
                    }
                }
                event = tray.recv_event(), if tray_mutations.is_empty() => {
                    let Some(event) = event else {
                        return Ok(());
                    };
                    tracing::debug!(?event, "tray event received");
                    match event {
                        AppMenuEvent::Quit => return Ok(()),
                        AppMenuEvent::ShowAbout => {
                            if let Err(error) = about_window.open() {
                                tracing::error!(%error, "About failed");
                            }
                        }
                        AppMenuEvent::ShowSshClientConfiguration => {
                            if ssh_client_configuration_window_open
                                .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                                .is_ok()
                            {
                                let prompt = Arc::clone(&agent_prompt);
                                let dialog = ssh_client_configuration_window.clone();
                                let open = Arc::clone(&ssh_client_configuration_window_open);
                                tokio::spawn(async move {
                                    let task = tokio::task::spawn_blocking(move || {
                                        let _owner_session = prompt.activate();
                                        if let Err(error) = dialog.open(prompt.hwnd().0 as usize) {
                                            tracing::error!(%error, "SSH client configuration failed");
                                        }
                                    });
                                    if let Err(error) = task.await {
                                        tracing::error!(%error, "SSH client configuration task failed");
                                    }
                                    open.store(false, Ordering::Release);
                                });
                            }
                        }
                        AppMenuEvent::OpenProfiles => {
                            if let Err(error) = Self::show_profiles(&runtime, &profiles_window) {
                                tracing::error!(%error, "could not open profiles window");
                            }
                        }
                        AppMenuEvent::ShowProfileDetails(id) => {
                            if let Err(error) = Self::show_profile_details(&runtime, &profiles_window, id) {
                                tracing::error!(%error, "could not open profile details");
                            }
                        }
                        AppMenuEvent::ShowTrustEntries(id) => {
                            if let Err(error) = Self::show_trust_entries(&runtime, &profiles_window, id) {
                                tracing::error!(%error, "could not open Public Keys window");
                            }
                        }
                        AppMenuEvent::AddProfile => {
                            let window = Arc::clone(&profiles_window);
                            if let Err(error) = runtime.invoke(move || window.add_profile()) {
                                tracing::error!(%error, "could not open profile setup");
                            }
                        }
                        AppMenuEvent::TestConnection(id) => {
                            let window = Arc::clone(&profiles_window);
                            if let Err(error) = runtime.invoke(move || window.test_connection(id)) {
                                tracing::error!(%error, "could not open connection test");
                            }
                        }
                        AppMenuEvent::SetProfileEnabled { id, enabled } => {
                            tray_mutations.push(ProfileMutation::SetEnabled { id, enabled }
                                .execute(Arc::clone(&application)));
                        }
                    }
                }
                changed = profile_changes.changed() => {
                    changed.map_err(|_| anyhow::anyhow!("profile change publisher stopped"))?;
                    let profiles = profile_changes.borrow_and_update().clone();
                    tray.post_profiles_update(profiles)?;
                }
            }
        }
    }

    fn show_profiles(
        runtime: &ReactorUiRuntime,
        profiles_window: &Arc<ProfilesWindow>,
    ) -> anyhow::Result<()> {
        let profiles_window = Arc::clone(profiles_window);
        runtime.invoke(move || profiles_window.show())
    }

    fn show_profile_details(
        runtime: &ReactorUiRuntime,
        profiles_window: &Arc<ProfilesWindow>,
        profile_id: ProfileId,
    ) -> anyhow::Result<()> {
        let profiles_window = Arc::clone(profiles_window);
        runtime.invoke(move || profiles_window.show_profile_details(profile_id))
    }

    fn show_trust_entries(
        runtime: &ReactorUiRuntime,
        profiles_window: &Arc<ProfilesWindow>,
        profile_id: ProfileId,
    ) -> anyhow::Result<()> {
        let profiles_window = Arc::clone(profiles_window);
        runtime.invoke(move || profiles_window.show_trust_entries(profile_id))
    }
}
