use keyringd_ui_core::app_menu::{AppMenuEvent, AppMenuLifecycleEvent};
use keyringd_ui_core::runtime::UiEventSender;
use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send, sel};
use objc2_app_kit::{
    NSApplication, NSApplicationActivationPolicy, NSApplicationDelegate,
    NSApplicationDidUpdateNotification, NSModalPanelWindowLevel, NSRequestUserAttentionType,
    NSWindow,
};
use objc2_foundation::{
    NSAppleEventManager, NSDistributedNotificationCenter, NSNotification, NSNotificationCenter,
    NSObject, NSObjectProtocol, NSString,
};
use std::rc::Rc;

use crate::app::ui::MainMenu;
use crate::app::ui::about_window::AboutWindow;

pub(crate) struct AppShellIvars {
    about: Rc<AboutWindow>,
    finder_request: std::path::PathBuf,
    menu: UiEventSender<AppMenuLifecycleEvent>,
    socket: std::path::PathBuf,
    socket_identity: Option<(u64, u64)>,
}

define_class!(
    /// Owns the application-wide window model.
    ///
    /// macOS orders an application's windows as a group and offers three ways to
    /// raise that group: the Dock icon, Cmd-Tab and the Window menu. All three
    /// exist only while the activation policy is `Regular`. The agent itself has
    /// no user interface, so the policy is `Accessory` while nothing is on screen
    /// and `Regular` for as long as any window is, which is exactly when a window
    /// can be covered by another application and needs a way back.
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "KeyringdAppShell"]
    #[ivars = AppShellIvars]
    pub(crate) struct AppShell;

    unsafe impl NSObjectProtocol for AppShell {}
    unsafe impl NSApplicationDelegate for AppShell {
        #[unsafe(method(applicationDidFinishLaunching:))]
        fn did_finish_launching(&self, _notification: &NSNotification) {
            // Apple supplies 'oapp' with 'prdt' (keyAEPropData) set to 'lgit'
            // (keyAELaunchedAsLogInItem) for login startup.
            // Inspect it here, once AppKit has dispatched the launch event.
            let at_login = NSAppleEventManager::sharedAppleEventManager()
                .currentAppleEvent()
                .is_some_and(|event| {
                    event.eventID() == u32::from_be_bytes(*b"oapp")
                        && event.paramDescriptorForKeyword(u32::from_be_bytes(*b"prdt"))
                            .is_some_and(|property| property.enumCodeValue() == u32::from_be_bytes(*b"lgit"))
                });
            if !at_login {
                self.open_profiles();
            }
        }
        #[unsafe(method(applicationWillTerminate:))]
        fn will_terminate(&self, _notification: &NSNotification) {
            use std::os::unix::fs::MetadataExt;
            if let Ok(metadata) = std::fs::symlink_metadata(&self.ivars().socket)
                && Some((metadata.dev(), metadata.ino())) == self.ivars().socket_identity
            {
                let _ = std::fs::remove_file(&self.ivars().socket);
            }
        }
        #[unsafe(method(applicationShouldHandleReopen:hasVisibleWindows:))]
        fn reopen(&self, _app: &NSApplication, has_visible_windows: bool) -> bool {
            if !has_visible_windows {
                self.open_profiles();
            }
            true
        }
    }

    impl AppShell {
        #[unsafe(method(keyringdOpenProfiles:))]
        fn finder_opened(&self, _notification: &NSNotification) {
            self.consume_finder_request();
        }
        #[unsafe(method(keyringdApplicationDidUpdate:))]
        fn application_did_update(&self, _notification: &NSNotification) {
            self.apply_activation_policy();
        }
    }
);

impl AppShell {
    pub(crate) fn install(
        mtm: MainThreadMarker,
        home: &std::path::Path,
        menu: UiEventSender<AppMenuLifecycleEvent>,
        socket: &std::path::Path,
    ) -> Retained<Self> {
        let about = AboutWindow::new(mtm);
        MainMenu::install(mtm, &about);
        let allocated = mtm.alloc::<Self>().set_ivars(AppShellIvars {
            about,
            finder_request: crate::app::finder_launch::request_file(home),
            menu,
            socket: socket.into(),
            socket_identity: {
                use std::os::unix::fs::MetadataExt;
                std::fs::symlink_metadata(socket)
                    .ok()
                    .map(|metadata| (metadata.dev(), metadata.ino()))
            },
        });
        let shell: Retained<Self> = unsafe { msg_send![super(allocated), init] };
        NSApplication::sharedApplication(mtm).setDelegate(Some(ProtocolObject::from_ref(&*shell)));
        // Windows go away in more ways than one: some are closed, some are only
        // ordered out, and a sheet ends through its parent. AppKit announces the
        // end of each pass over the event loop, which is the one moment that
        // covers all of them.
        unsafe {
            NSDistributedNotificationCenter::defaultCenter().addObserver_selector_name_object(
                &shell,
                sel!(keyringdOpenProfiles:),
                Some(&NSString::from_str(
                    crate::app::finder_launch::OPEN_PROFILES,
                )),
                None,
            );
            NSNotificationCenter::defaultCenter().addObserver_selector_name_object(
                &shell,
                sel!(keyringdApplicationDidUpdate:),
                Some(NSApplicationDidUpdateNotification),
                None,
            );
        }
        shell.consume_finder_request();
        shell.apply_activation_policy();
        shell
    }

    fn consume_finder_request(&self) {
        match std::fs::remove_file(&self.ivars().finder_request) {
            Ok(()) => self.open_profiles(),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => tracing::warn!(%error, "could not read the Finder request"),
        }
    }

    fn open_profiles(&self) {
        self.ivars()
            .menu
            .send(AppMenuLifecycleEvent::ActionRequested(
                AppMenuEvent::OpenProfiles,
            ));
    }

    /// Brings `window` on screen and keeps the application reachable while it is.
    pub(crate) fn present(&self, window: &NSWindow) {
        self.activate();
        window.makeKeyAndOrderFront(None);
    }

    /// Shows the About window for a request that did not come from a window,
    /// such as the status bar menu, which is reachable while the agent is idle
    /// and the application menu that also carries About does not exist.
    pub(crate) fn show_about(&self) {
        self.activate();
        self.ivars().about.show();
    }

    /// Prepares the application to put a window on screen, for components that
    /// order their own window front. Call this first so the Dock icon and the
    /// Window menu exist by the time the window appears.
    pub(crate) fn activate_for_window(&self) {
        self.activate();
    }

    /// The window a prompt raised right now belongs to, if any.
    ///
    /// Only an available document window can own a sheet. If every window is
    /// already busy with a sheet, background authentication uses its own window
    /// instead of attaching a second sheet to the first one.
    pub(crate) fn anchor(&self) -> Option<Retained<NSWindow>> {
        NSApplication::sharedApplication(self.mtm())
            .orderedWindows()
            .iter()
            .find(|window| {
                Self::is_user_facing(window)
                    && window.sheetParent().is_none()
                    && window.attachedSheet().is_none()
            })
    }

    /// Reports that a window took itself off screen.
    ///
    /// An application that has been idle sees no events, and so no update pass
    /// either. A prompt that the agent takes down on its own behalf, once the
    /// phone has answered, is exactly that case.
    pub(crate) fn window_hidden(&self) {
        self.apply_activation_policy();
    }

    /// Asks for attention on behalf of an operation the user did not start from a
    /// window, such as a signature `ssh` requested from a terminal. Bouncing the
    /// Dock icon only means anything once the policy is `Regular`.
    pub(crate) fn request_attention(&self) {
        self.activate();
        let application = NSApplication::sharedApplication(self.mtm());
        application.requestUserAttention(NSRequestUserAttentionType::CriticalRequest);
    }

    fn activate(&self) {
        let application = NSApplication::sharedApplication(self.mtm());
        application.setActivationPolicy(NSApplicationActivationPolicy::Regular);
        // `activate()` is only available on newer macOS releases. The application
        // supports macOS 13, where this selector is the compatible equivalent.
        #[allow(deprecated)]
        application.activateIgnoringOtherApps(true);
    }

    fn apply_activation_policy(&self) {
        let application = NSApplication::sharedApplication(self.mtm());
        let wanted = if application
            .windows()
            .iter()
            .any(|window| Self::is_user_facing(&window))
        {
            NSApplicationActivationPolicy::Regular
        } else {
            NSApplicationActivationPolicy::Accessory
        };
        // This runs once per pass over the event loop, and changing the policy
        // makes the Dock add or remove an icon.
        if application.activationPolicy() != wanted {
            application.setActivationPolicy(wanted);
        }
    }

    /// `NSApplication` reports every window the process owns, including the menu
    /// bar item's own window and any open menu. Only windows at or below the
    /// modal panel level are ones a person can be left looking at.
    fn is_user_facing(window: &Retained<NSWindow>) -> bool {
        window.isVisible() && window.level() <= NSModalPanelWindowLevel
    }
}
