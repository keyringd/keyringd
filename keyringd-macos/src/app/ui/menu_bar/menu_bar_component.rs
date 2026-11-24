use keyringd_core::model::Profile;
use keyringd_ui_core::runtime::{UiEventReceiver, UiEventSender};
use keyringd_ui_core::text as ui_text;
use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2::{AnyThread, MainThreadOnly, sel};
use objc2_app_kit::{
    NSImage, NSMenu, NSMenuItem, NSStatusBar, NSStatusItem, NSUserInterfaceItemIdentification,
    NSVariableStatusItemLength,
};
use objc2_foundation::{NSData, NSString};

use super::{MenuBarActionTarget, MenuBarCommandHandler};
use keyringd_ui_core::app_menu::{AppMenuEvent, AppMenuLifecycleEvent, AppMenuState};
use keyringd_ui_core::runtime::ComponentRuntime;

pub(crate) struct MenuBarComponent {
    item: Option<Retained<NSStatusItem>>,
    state: ComponentRuntime<AppMenuState>,
    menu: Retained<NSMenu>,
    events: UiEventSender<AppMenuLifecycleEvent>,
    targets: Vec<Retained<MenuBarActionTarget>>,
}

impl MenuBarComponent {
    pub(crate) fn mount(
        profiles: Vec<Profile>,
        mtm: MainThreadMarker,
    ) -> anyhow::Result<(Self, UiEventReceiver<AppMenuLifecycleEvent>)> {
        let (events, receiver) = UiEventSender::channel();
        let menu = NSMenu::new(mtm);
        let item = NSStatusBar::systemStatusBar().statusItemWithLength(NSVariableStatusItemLength);
        let Some(button) = item.button(mtm) else {
            NSStatusBar::systemStatusBar().removeStatusItem(&item);
            return Err(anyhow::anyhow!("macOS did not create a status bar button"));
        };
        // Vector template derived from packaging/windows/keyringd-icon.svg.
        let pdf = include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../packaging/macos/keyringd-menu.pdf"
        ));
        let data = NSData::with_bytes(pdf);
        if let Some(icon) = NSImage::initWithData(NSImage::alloc(), &data) {
            icon.setTemplate(true);
            icon.setAccessibilityDescription(Some(&NSString::from_str("Keyringd SSH agent")));
            button.setImage(Some(&icon));
        } else {
            button.setTitle(&NSString::from_str("Keyringd"));
        }
        item.setMenu(Some(&menu));
        #[cfg(feature = "test-support")]
        crate::app::ui_test_support::ax_driver::register_status_menu(menu.clone());
        let mut component = Self {
            item: Some(item),
            state: ComponentRuntime::new(AppMenuState::default()),
            menu,
            events,
            targets: Vec::new(),
        };
        component.dispatch(AppMenuLifecycleEvent::Mounted(profiles), mtm);
        Ok((component, receiver))
    }

    pub(crate) fn events(&self) -> UiEventSender<AppMenuLifecycleEvent> {
        self.events.clone()
    }

    pub(crate) fn dispatch(
        &mut self,
        event: AppMenuLifecycleEvent,
        mtm: MainThreadMarker,
    ) -> Option<AppMenuEvent> {
        let command = self.state.dispatch(event)?;
        MenuBarCommandHandler::handle(command, self, mtm)
    }

    pub(crate) fn stop(&mut self, mtm: MainThreadMarker) {
        self.dispatch(AppMenuLifecycleEvent::StopRequested, mtm);
    }

    pub(super) fn remove(&mut self) {
        self.menu.removeAllItems();
        self.targets.clear();
        if let Some(item) = self.item.take() {
            NSStatusBar::systemStatusBar().removeStatusItem(&item);
        }
    }

    pub(super) fn render(&mut self, profiles: &[Profile], mtm: MainThreadMarker) {
        self.menu.removeAllItems();
        self.targets.clear();
        self.add_root(
            "status-menu.getting-started",
            ui_text::APP_MENU_SSH_CLIENT_CONFIGURATION,
            AppMenuEvent::ShowSshClientConfiguration,
            mtm,
        );
        self.menu.addItem(&NSMenuItem::separatorItem(mtm));
        for profile in profiles {
            let submenu = NSMenu::new(mtm);
            self.add_to(
                &submenu,
                keyringd_ui_core::profile_list_item::enable_action(profile.is_enabled()),
                AppMenuEvent::SetProfileEnabled {
                    id: profile.id().clone(),
                    enabled: !profile.is_enabled(),
                },
                mtm,
            );
            self.add_to(
                &submenu,
                ui_text::PROFILE_LIST_ITEM_DETAILS,
                AppMenuEvent::ShowProfileDetails(profile.id().clone()),
                mtm,
            );
            self.add_to(
                &submenu,
                ui_text::TRUST_ENTRIES_MENU_ACTION,
                AppMenuEvent::ShowTrustEntries(profile.id().clone()),
                mtm,
            );
            let test = self.add_to(
                &submenu,
                ui_text::PROFILE_LIST_ITEM_TEST_CONNECTION,
                AppMenuEvent::TestConnection(profile.id().clone()),
                mtm,
            );
            test.setEnabled(profile.is_enabled());
            let root = unsafe {
                NSMenuItem::initWithTitle_action_keyEquivalent(
                    NSMenuItem::alloc(mtm),
                    &NSString::from_str(profile.display_name()),
                    None,
                    &NSString::new(),
                )
            };
            root.setSubmenu(Some(&submenu));
            self.menu.addItem(&root);
        }
        self.menu.addItem(&NSMenuItem::separatorItem(mtm));
        self.add_root(
            "status-menu.profiles",
            ui_text::APP_MENU_PROFILES,
            AppMenuEvent::OpenProfiles,
            mtm,
        );
        self.add_root(
            "status-menu.add-profile",
            ui_text::APP_MENU_ADD_PROFILE,
            AppMenuEvent::AddProfile,
            mtm,
        );
        self.menu.addItem(&NSMenuItem::separatorItem(mtm));
        // The application menu that also carries About exists only while a
        // window is open, so the status bar menu is the only way to reach it
        // from an idle agent.
        self.add_root(
            "status-menu.about",
            ui_text::APP_MENU_ABOUT,
            AppMenuEvent::ShowAbout,
            mtm,
        );
        self.add_root(
            "status-menu.quit",
            ui_text::APP_MENU_QUIT,
            AppMenuEvent::Quit,
            mtm,
        );
    }

    fn add_root(
        &mut self,
        identifier: &str,
        title: &str,
        event: AppMenuEvent,
        mtm: MainThreadMarker,
    ) {
        let menu = self.menu.clone();
        let item = self.add_to(&menu, title, event, mtm);
        item.setIdentifier(Some(&NSString::from_str(identifier)));
    }

    fn add_to(
        &mut self,
        menu: &NSMenu,
        title: &str,
        event: AppMenuEvent,
        mtm: MainThreadMarker,
    ) -> Retained<NSMenuItem> {
        let target = MenuBarActionTarget::new(mtm, self.events.clone(), event);
        let item = unsafe {
            NSMenuItem::initWithTitle_action_keyEquivalent(
                NSMenuItem::alloc(mtm),
                &NSString::from_str(title),
                Some(sel!(performAction:)),
                &NSString::new(),
            )
        };
        unsafe { item.setTarget(Some(&target)) };
        menu.addItem(&item);
        self.targets.push(target);
        item
    }
}

impl Drop for MenuBarComponent {
    fn drop(&mut self) {
        self.stop(MainThreadMarker::new().expect("the menu bar is owned by the AppKit thread"));
    }
}
