use crate::app::ui::util::load_tray_icon;
use keyringd_ui_core::app_menu::AppMenuEvent;
use keyringd_ui_core::text as ui_text;

use tray_icon::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu};
use tray_icon::{TrayIcon, TrayIconBuilder};

pub(crate) struct TrayMenuView;

impl TrayMenuView {
    pub(crate) fn render(
        profiles: Vec<keyringd_core::model::Profile>,
    ) -> anyhow::Result<(TrayIcon, Vec<(String, AppMenuEvent)>)> {
        let menu = Menu::new();
        let ssh_client_configuration_window =
            MenuItem::new(ui_text::APP_MENU_SSH_CLIENT_CONFIGURATION, true, None);
        let manage = MenuItem::new(ui_text::APP_MENU_PROFILES, true, None);
        let add = MenuItem::new(ui_text::APP_MENU_ADD_PROFILE, true, None);
        let exit = MenuItem::new(ui_text::APP_MENU_EXIT, true, None);
        let about = MenuItem::new(ui_text::APP_MENU_ABOUT, true, None);
        menu.append_items(&[
            &ssh_client_configuration_window,
            &PredefinedMenuItem::separator(),
        ])?;
        let mut actions = vec![
            (
                ssh_client_configuration_window.id().as_ref().to_owned(),
                AppMenuEvent::ShowSshClientConfiguration,
            ),
            (manage.id().as_ref().to_owned(), AppMenuEvent::OpenProfiles),
            (add.id().as_ref().to_owned(), AppMenuEvent::AddProfile),
            (exit.id().as_ref().to_owned(), AppMenuEvent::Quit),
            (about.id().as_ref().to_owned(), AppMenuEvent::ShowAbout),
        ];
        for profile in &profiles {
            let submenu = Submenu::new(profile.display_name(), true);
            let enabled = CheckMenuItem::new(
                ui_text::PROFILE_LIST_ITEM_ENABLED,
                true,
                profile.is_enabled(),
                None,
            );
            let details = MenuItem::new(ui_text::PROFILE_LIST_ITEM_DETAILS, true, None);
            let trust_entries = MenuItem::new(ui_text::TRUST_ENTRIES_MENU_ACTION, true, None);
            let test = MenuItem::new(
                ui_text::PROFILE_LIST_ITEM_TEST_CONNECTION,
                profile.is_enabled(),
                None,
            );
            submenu.append_items(&[
                &enabled,
                &PredefinedMenuItem::separator(),
                &details,
                &trust_entries,
                &test,
            ])?;
            menu.append(&submenu)?;
            let id = profile.id().clone();
            actions.extend([
                (
                    details.id().as_ref().to_owned(),
                    AppMenuEvent::ShowProfileDetails(id.clone()),
                ),
                (
                    enabled.id().as_ref().to_owned(),
                    AppMenuEvent::SetProfileEnabled {
                        id: id.clone(),
                        enabled: !profile.is_enabled(),
                    },
                ),
                (
                    trust_entries.id().as_ref().to_owned(),
                    AppMenuEvent::ShowTrustEntries(id.clone()),
                ),
                (
                    test.id().as_ref().to_owned(),
                    AppMenuEvent::TestConnection(id),
                ),
            ]);
        }
        menu.append_items(&[
            &PredefinedMenuItem::separator(),
            &manage,
            &add,
            &PredefinedMenuItem::separator(),
            &about,
            &exit,
        ])?;
        let builder = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_tooltip("Keyringd — hardware-protected SSH")
            .with_icon(load_tray_icon()?);
        #[cfg(feature = "test-support")]
        let builder = if let Ok(guid) = std::env::var("KEYRINGD_UI_TEST_TRAY_GUID") {
            builder.with_guid(windows::core::GUID::try_from(guid.as_str())?.to_u128())
        } else {
            builder
        };
        Ok((builder.build()?, actions))
    }
}
