use objc2::runtime::Sel;
use objc2::{MainThreadMarker, MainThreadOnly, sel};
use objc2_app_kit::{NSApplication, NSMenu, NSMenuItem, NSUserInterfaceItemIdentification};
use objc2_foundation::NSString;

use keyringd_ui_core::text as ui_text;

/// The menu bar the application shows whenever its activation policy is
/// `Regular`.
///
/// An application with a Dock icon is expected to have one, and the Window menu
/// in particular is how macOS lets someone find a window that another
/// application has covered. Standard editing commands use the responder chain;
/// About targets the application-owned About component.
pub(super) struct MainMenu;

impl MainMenu {
    pub(super) fn install(
        mtm: MainThreadMarker,
        about: &crate::app::ui::about_window::AboutWindow,
    ) {
        let application = NSApplication::sharedApplication(mtm);
        let main = NSMenu::new(mtm);

        let app_menu = Self::submenu(&main, "Keyringd", mtm);
        main.itemAtIndex(0)
            .expect("the application submenu should be at index zero because it was added first")
            .setIdentifier(Some(&NSString::from_str("main-menu.application")));
        Self::item(
            &app_menu,
            ui_text::APP_MENU_ABOUT,
            sel!(showAbout:),
            "",
            mtm,
        );
        app_menu
            .itemAtIndex(0)
            .expect("the About item should be at index zero because it was added first to the application submenu")
            .setIdentifier(Some(&NSString::from_str("main-menu.about")));
        about.bind_menu_item(
            &app_menu.itemAtIndex(0)
                .expect("the About item should be at index zero because it was added first to the application submenu"),
        );
        app_menu.addItem(&NSMenuItem::separatorItem(mtm));
        Self::item(&app_menu, "Hide Keyringd", sel!(hide:), "h", mtm);
        Self::item(
            &app_menu,
            "Hide Others",
            sel!(hideOtherApplications:),
            "",
            mtm,
        );
        Self::item(&app_menu, "Show All", sel!(unhideAllApplications:), "", mtm);
        app_menu.addItem(&NSMenuItem::separatorItem(mtm));
        Self::item(
            &app_menu,
            ui_text::APP_MENU_QUIT,
            sel!(terminate:),
            "q",
            mtm,
        );

        let file_menu = Self::submenu(&main, "File", mtm);
        Self::item(&file_menu, ui_text::CLOSE, sel!(performClose:), "w", mtm);

        let edit_menu = Self::submenu(&main, "Edit", mtm);
        Self::item(&edit_menu, "Undo", sel!(undo:), "z", mtm);
        Self::item(&edit_menu, "Redo", sel!(redo:), "Z", mtm);
        edit_menu.addItem(&NSMenuItem::separatorItem(mtm));
        Self::item(&edit_menu, "Cut", sel!(cut:), "x", mtm);
        Self::item(&edit_menu, "Copy", sel!(copy:), "c", mtm);
        Self::item(&edit_menu, "Paste", sel!(paste:), "v", mtm);
        Self::item(&edit_menu, "Select All", sel!(selectAll:), "a", mtm);

        let window_menu = Self::submenu(&main, "Window", mtm);
        Self::item(
            &window_menu,
            "Minimize",
            sel!(performMiniaturize:),
            "m",
            mtm,
        );
        Self::item(&window_menu, "Zoom", sel!(performZoom:), "", mtm);
        window_menu.addItem(&NSMenuItem::separatorItem(mtm));
        Self::item(
            &window_menu,
            "Bring All to Front",
            sel!(arrangeInFront:),
            "",
            mtm,
        );

        application.setMainMenu(Some(&main));
        // AppKit appends an entry per titled window below the items added above,
        // which is the standard way back to a window that has been covered.
        application.setWindowsMenu(Some(&window_menu));
    }

    fn submenu(main: &NSMenu, title: &str, mtm: MainThreadMarker) -> objc2::rc::Retained<NSMenu> {
        let title = NSString::from_str(title);
        let menu = NSMenu::initWithTitle(NSMenu::alloc(mtm), &title);
        let item = NSMenuItem::new(mtm);
        item.setTitle(&title);
        item.setSubmenu(Some(&menu));
        main.addItem(&item);
        menu
    }

    fn item(menu: &NSMenu, title: &str, action: Sel, key: &str, mtm: MainThreadMarker) {
        let item = unsafe {
            NSMenuItem::initWithTitle_action_keyEquivalent(
                NSMenuItem::alloc(mtm),
                &NSString::from_str(title),
                Some(action),
                &NSString::from_str(key),
            )
        };
        menu.addItem(&item);
    }
}
