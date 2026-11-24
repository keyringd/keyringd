use gtk4::prelude::{AccessibleExtManual, BoxExt, ButtonExt, ObjectExt, PopoverExt};
use keyringd_ui_core::text as ui_text;

/// Application actions with the same accessible controls as profile actions.
pub(super) struct ApplicationMenu;

impl ApplicationMenu {
    pub(super) fn build() -> gtk4::MenuButton {
        let actions = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .accessible_role(gtk4::AccessibleRole::Group)
            .margin_top(6)
            .margin_bottom(6)
            .margin_start(6)
            .margin_end(6)
            .build();
        actions.update_property(&[gtk4::accessible::Property::Label(
            ui_text::APP_MENU_ACCESSIBILITY,
        )]);
        let popover = gtk4::Popover::builder().child(&actions).build();
        for (label, action) in [
            (
                ui_text::APP_MENU_SSH_CLIENT_CONFIGURATION,
                "win.open-ssh-setup",
            ),
            (ui_text::APP_MENU_ABOUT, "win.about"),
        ] {
            let button = gtk4::Button::builder()
                .label(label)
                .action_name(action)
                .css_classes(["flat"])
                .halign(gtk4::Align::Fill)
                .build();
            let weak_popover = popover.downgrade();
            button.connect_clicked(move |_| {
                if let Some(popover) = weak_popover.upgrade() {
                    popover.popdown();
                }
            });
            actions.append(&button);
        }
        let icon = gtk4::Image::builder()
            .icon_name("open-menu-symbolic")
            .accessible_role(gtk4::AccessibleRole::Presentation)
            .build();
        let menu = gtk4::MenuButton::builder()
            .child(&icon)
            .tooltip_text(ui_text::APP_MENU_TOOLTIP)
            .popover(&popover)
            .primary(true)
            .build();
        menu.update_property(&[gtk4::accessible::Property::Label(ui_text::APP_MENU_TOOLTIP)]);
        menu
    }
}
