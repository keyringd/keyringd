use keyringd_ui_core::runtime::{UiEventReceiver, UiEventSender, UiEventTarget};
use keyringd_ui_core::text as ui_text;
use objc2::rc::Retained;
use objc2::{MainThreadMarker, MainThreadOnly, sel};
use objc2_app_kit::{
    NSAccessibility, NSBackingStoreType, NSButton, NSCellImagePosition, NSFont, NSImage, NSWindow,
    NSWindowStyleMask, NSWindowToolbarStyle,
};
use objc2_foundation::{NSPoint, NSRect, NSSize, NSString};

use crate::app::ui::profile_list::ProfileListComponent;
use crate::app::ui::profile_list::ProfileListProps;
use crate::app::ui::{ComponentRuntime, ControlActionTarget, Layout};

use super::ProfilesWindowCommand;
use super::ProfilesWindowEvent;
use super::ProfilesWindowState;

pub(crate) struct ProfilesWindow {
    window: Retained<NSWindow>,
    list: ProfileListComponent,
    _configure_ssh_target: Retained<ControlActionTarget>,
    runtime: ComponentRuntime<ProfilesWindowState>,
}

impl ProfilesWindow {
    pub(crate) fn new(
        initial_profiles: Vec<keyringd_core::model::Profile>,
        configure_ssh: UiEventTarget<()>,
        mtm: MainThreadMarker,
    ) -> (Self, UiEventReceiver<ProfilesWindowEvent>) {
        let (events, receiver) = UiEventSender::channel();
        let window = unsafe {
            NSWindow::initWithContentRect_styleMask_backing_defer(
                NSWindow::alloc(mtm),
                NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(760.0, 520.0)),
                NSWindowStyleMask::Titled
                    | NSWindowStyleMask::Closable
                    | NSWindowStyleMask::Miniaturizable
                    | NSWindowStyleMask::Resizable,
                NSBackingStoreType::Buffered,
                false,
            )
        };
        window.setTitle(&NSString::from_str("Keyringd"));
        window.setMinSize(NSSize::new(560.0, 400.0));
        window.setToolbarStyle(NSWindowToolbarStyle::Unified);
        unsafe { window.setReleasedWhenClosed(false) };
        window.center();
        let content = window.contentView().expect(
            "the initialized Profiles window should have the content view supplied by AppKit",
        );
        let list = ProfileListComponent::new(
            keyringd_ui_core::runtime::UiEventTarget::from_sender(events)
                .map(ProfilesWindowEvent::from),
            mtm,
        );
        window.setToolbar(Some(list.toolbar()));
        content.addSubview(list.view());
        Layout::pin_top(list.view(), &content);
        let configure_ssh_target = ControlActionTarget::new(mtm, configure_ssh, ());
        let configure_ssh = unsafe {
            NSButton::buttonWithTitle_target_action(
                &NSString::from_str(ui_text::APP_MENU_SSH_CLIENT_CONFIGURATION),
                Some(&configure_ssh_target),
                Some(sel!(performAction:)),
                mtm,
            )
        };
        configure_ssh.setBordered(false);
        configure_ssh.setFont(Some(&NSFont::systemFontOfSize(12.0)));
        let gear = NSImage::imageWithSystemSymbolName_accessibilityDescription(
            &NSString::from_str("gearshape"),
            None,
        );
        configure_ssh.setImage(gear.as_deref());
        configure_ssh.setImagePosition(NSCellImagePosition::ImageLeft);
        configure_ssh.setImageHugsTitle(true);
        configure_ssh.setAccessibilityIdentifier(Some(&NSString::from_str(
            "profiles.configure-ssh-clients",
        )));
        content.addSubview(&configure_ssh);
        Layout::prepare(&configure_ssh);
        Layout::height(&configure_ssh, 24.0);
        configure_ssh
            .leadingAnchor()
            .constraintEqualToAnchor_constant(&content.leadingAnchor(), 24.0)
            .setActive(true);
        configure_ssh
            .bottomAnchor()
            .constraintEqualToAnchor_constant(&content.bottomAnchor(), -12.0)
            .setActive(true);
        list.view()
            .bottomAnchor()
            .constraintEqualToAnchor(&configure_ssh.topAnchor())
            .setActive(true);
        let mut component = Self {
            window,
            list,
            _configure_ssh_target: configure_ssh_target,
            runtime: ComponentRuntime::new(ProfilesWindowState::new(initial_profiles)),
        };
        component.render(mtm);
        (component, receiver)
    }

    pub(crate) fn dispatch(
        &mut self,
        event: ProfilesWindowEvent,
        mtm: MainThreadMarker,
    ) -> Option<ProfilesWindowCommand> {
        let list = &mut self.list;
        self.runtime.dispatch_and_render(event, |state| {
            list.render(
                ProfileListProps {
                    saving: state.is_saving(),
                    creation_enabled: state.can_add_profile(),
                    revision: state.profiles_revision(),
                    profiles: state.profiles().to_vec(),
                    editing: state
                        .editing()
                        .map(|(id, name)| (id.clone(), name.to_owned())),
                },
                state.device_key_error(),
                mtm,
            );
        })
    }

    fn render(&mut self, mtm: MainThreadMarker) {
        self.list.render(
            ProfileListProps {
                saving: self.runtime.state().is_saving(),
                creation_enabled: self.runtime.state().can_add_profile(),
                revision: self.runtime.state().profiles_revision(),
                profiles: self.runtime.state().profiles().to_vec(),
                editing: self
                    .runtime
                    .state()
                    .editing()
                    .map(|(id, name)| (id.clone(), name.to_owned())),
            },
            self.runtime.state().device_key_error(),
            mtm,
        );
    }

    pub(crate) fn window_handle(&self) -> Retained<NSWindow> {
        self.window.clone()
    }
}
