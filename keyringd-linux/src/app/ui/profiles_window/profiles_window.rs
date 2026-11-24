use crate::app::ui::profiles_window::ApplicationMenu;
use crate::app::ui::toast::ToastOverlay;
use crate::system::open_ssh::LinuxOpenSshService;
use keyringd_ui_core::runtime::UiEventTarget;

use crate::app::ui::profiles_window::ProfilesWindowCommandHandler;
use keyringd_ui_core::profiles_window::{ProfilesWindowEvent, ProfilesWindowState};

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use crate::app::ui::ComponentRuntime;
use crate::app::ui::UiEventReceiver;
use crate::app::ui::profile_list::ProfileListView;
use crate::app::ui::ssh_client_configuration_window::SshClientConfigurationWindow;
use crate::app::{APPLICATION_ID, LinuxApplication};
use futures_util::FutureExt;
use gtk4::gio;
use gtk4::gio::prelude::ActionMapExt;
use gtk4::glib::object::{Cast, ObjectExt};
use gtk4::prelude::{BoxExt, GtkWindowExt, WidgetExt};
use keyringd_ui_core::key_operation::KeyOperationCoordinator;
use keyringd_ui_core::profile_list::ProfileListEvent;
use keyringd_ui_core::text as ui_text;

pub(crate) struct ProfilesWindow;

impl ProfilesWindow {
    pub(crate) fn present(
        application: &libadwaita::Application,
        profile_application: &Arc<LinuxApplication>,
        open_ssh: &Arc<LinuxOpenSshService>,
        prompts: &Arc<KeyOperationCoordinator>,
    ) -> libadwaita::ApplicationWindow {
        let (initial_profiles, mut profile_changes) = UiEventReceiver::from_tokio_watch(
            &tokio::runtime::Handle::current(),
            profile_application.subscribe_profile_changes(),
        );
        let runtime = Rc::new(ComponentRuntime::new(ProfilesWindowState::new(
            initial_profiles,
        )));
        let toast_overlay = ToastOverlay::new();
        let on_toast = toast_overlay.event_target();
        let (mutation_events, mut mutation_results) =
            keyringd_ui_core::runtime::UiEventSender::channel();
        let command_handler = Rc::new(ProfilesWindowCommandHandler::new(
            application.clone(),
            Arc::clone(profile_application),
            Arc::clone(open_ssh),
            Arc::clone(prompts),
            on_toast.clone(),
            mutation_events,
        ));
        let window_holder = Rc::new(RefCell::new(None::<gtk4::Window>));
        let profile_list_view: Rc<ProfileListView> = Rc::new_cyclic(
            |weak_list: &std::rc::Weak<ProfileListView>| {
                let weak_list = weak_list.clone();
                let runtime = Rc::clone(&runtime);
                let window_holder = Rc::clone(&window_holder);
                let command_handler = Rc::clone(&command_handler);
                let on_event = UiEventTarget::new(move |event| {
                    let Some(profile_list_view) = weak_list.upgrade() else {
                        return;
                    };
                    let Some(parent) = window_holder.borrow().clone() else {
                        return;
                    };
                    // The GTK entry already owns the pixels for the current edit. Keep the draft
                    // in component state, but do not destroy and recreate the complete list for
                    // every keystroke.
                    let rebuild_list = !matches!(
                        &event,
                        ProfileListEvent::ProfileAction(action)
                            if matches!(
                                action.as_ref(),
                                keyringd_ui_core::profile_list_item::ProfileListItemEvent::RenameNameChanged { .. }
                            )
                    );
                    let event = match event {
                        ProfileListEvent::AddRequested => ProfilesWindowEvent::AddRequested,
                        ProfileListEvent::ProfileAction(action) => {
                            ProfilesWindowEvent::ProfileAction(action)
                        }
                    };
                    let command_to_handle = runtime.dispatch(event);
                    if rebuild_list {
                        let state = runtime.state();
                        profile_list_view.render(
                            state.profiles(),
                            state.editing(),
                            state.is_saving(),
                            state.can_add_profile(),
                        );
                    }
                    let Some(command_to_handle) = command_to_handle else {
                        return;
                    };
                    command_handler.handle(command_to_handle, &parent);
                });
                ProfileListView::new(on_event)
            },
        );
        profile_list_view.render(
            runtime.state().profiles(),
            runtime.state().editing(),
            runtime.state().is_saving(),
            runtime.state().can_add_profile(),
        );
        let toolbar = libadwaita::ToolbarView::new();
        let header = libadwaita::HeaderBar::new();
        let app_menu = self::ApplicationMenu::build();
        header.pack_end(&app_menu);
        toolbar.add_top_bar(&header);
        let hardware_banner = libadwaita::Banner::new("");
        hardware_banner.set_revealed(false);
        let handler_for_banner = Rc::clone(&command_handler);
        hardware_banner.connect_button_clicked(move |banner| {
            banner.set_sensitive(false);
            let banner = banner.downgrade();
            handler_for_banner.request_tpm_access(move || {
                if let Some(banner) = banner.upgrade() {
                    banner.set_sensitive(true);
                }
            });
        });
        let content = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        content.append(&hardware_banner);
        content.append(profile_list_view.widget());
        profile_list_view.widget().set_vexpand(true);
        let configure_ssh = gtk4::Button::builder()
            .child(
                &libadwaita::ButtonContent::builder()
                    .icon_name("preferences-system-symbolic")
                    .label(ui_text::APP_MENU_SSH_CLIENT_CONFIGURATION)
                    .build(),
            )
            .action_name("win.open-ssh-setup")
            .css_classes(["flat"])
            .halign(gtk4::Align::Start)
            .margin_start(24)
            .margin_bottom(12)
            .build();
        content.append(&configure_ssh);
        toast_overlay.widget().set_child(Some(&content));
        toolbar.set_content(Some(toast_overlay.widget()));
        let window = libadwaita::ApplicationWindow::builder()
            .application(application)
            .title("Keyringd")
            .icon_name(APPLICATION_ID)
            .default_width(640)
            .default_height(480)
            .content(&toolbar)
            .build();
        *window_holder.borrow_mut() = Some(window.clone().upcast());
        let open_ssh_setup = gio::SimpleAction::new("open-ssh-setup", None);
        let window_for_open_ssh_setup: gtk4::Window = window.clone().upcast();
        let open_ssh_for_setup = Arc::clone(open_ssh);
        open_ssh_setup.connect_activate(move |_, _| {
            SshClientConfigurationWindow::present(
                &window_for_open_ssh_setup,
                Arc::clone(&open_ssh_for_setup),
            );
        });
        window.add_action(&open_ssh_setup);
        let about = gio::SimpleAction::new("about", None);
        let window_for_about: gtk4::Window = window.clone().upcast();
        about.connect_activate(move |_, _| {
            crate::app::ui::about_window::AboutWindow::present(&window_for_about);
        });
        window.add_action(&about);
        let profile_list_view_for_changes = Rc::clone(&profile_list_view);
        let runtime_for_changes = Rc::clone(&runtime);
        let (window_closed, window_closed_receiver) = futures_channel::oneshot::channel();
        let window_closed = Rc::new(RefCell::new(Some(window_closed)));
        window.connect_destroy(move |_| {
            if let Some(window_closed) = window_closed.borrow_mut().take() {
                let _ = window_closed.send(());
            }
        });
        let application_for_results = Arc::clone(profile_application);
        let parent_for_results = window.downgrade();
        let handler_for_results = Rc::clone(&command_handler);
        gtk4::glib::MainContext::ref_thread_default().spawn_local(async move {
            let window_closed_receiver = window_closed_receiver.fuse();
            futures_util::pin_mut!(window_closed_receiver);
            loop {
                futures_util::select! {
                    profiles = profile_changes.receive().fuse() => {
                        let Some(_) = profiles else { break };
                        let event = ProfilesWindowEvent::ProfilesChanged(application_for_results.profiles());
                        runtime_for_changes.dispatch_and_render(event, |state| {
                            profile_list_view_for_changes
                                .render(state.profiles(), state.editing(), state.is_saving(), state.can_add_profile());
                        });
                    }
                    result = mutation_results.receive().fuse() => {
                        let Some(event) = result else { break };
                        // A toast must not rebuild entries and interrupt an active rename.
                        let refresh_list = !matches!(&event, ProfilesWindowEvent::ToastRequested(_));
                        if refresh_list {
                            runtime_for_changes.dispatch(ProfilesWindowEvent::ProfilesChanged(application_for_results.profiles()));
                        }
                        let command = runtime_for_changes.dispatch_and_render(event, |state| {
                            if refresh_list {
                                profile_list_view_for_changes.render(state.profiles(), state.editing(), state.is_saving(), state.can_add_profile());
                                hardware_banner.set_title(state.device_key_error().unwrap_or(""));
                                hardware_banner.set_button_label(
                                    state.can_grant_device_key_access().then_some("Grant TPM access…"),
                                );
                                hardware_banner.set_revealed(state.device_key_error().is_some());
                            }
                        });
                        if let (Some(command), Some(parent)) = (command, parent_for_results.upgrade()) {
                            handler_for_results.handle(command, &parent.upcast());
                        }
                    }
                    _ = window_closed_receiver => break,
                }
            }
        });
        let command = runtime
            .dispatch(ProfilesWindowEvent::Opened)
            .expect("opening checks Device Key availability");
        command_handler.handle(command, window.upcast_ref());
        window.present();
        window
    }
}
