use gtk4::glib::object::Cast;
use gtk4::prelude::{AccessibleExtManual, CheckButtonExt};
use gtk4::prelude::{BoxExt, WidgetExt};
use libadwaita::prelude::{ActionRowExt, PreferencesGroupExt};

use keyringd_ui_core::text as ui_text;

use crate::app::ui::page_header::PageHeaderView;
use crate::app::ui::radio_button::RadioButtonView;

pub(in crate::app::ui) struct ProfileFormPage {
    pub(in crate::app::ui) root: gtk4::Widget,
    pub(in crate::app::ui) name: libadwaita::EntryRow,
    pub(in crate::app::ui) create_passkey: RadioButtonView,
    pub(in crate::app::ui) existing_passkey: RadioButtonView,
    pub(in crate::app::ui) sk_format: RadioButtonView,
    pub(in crate::app::ui) ecdsa_format: RadioButtonView,
    pub(in crate::app::ui) advanced: gtk4::Expander,
    pub(in crate::app::ui) status: gtk4::Label,
    pub(in crate::app::ui) cancel: gtk4::Button,
    pub(in crate::app::ui) create: gtk4::Button,
    pub(in crate::app::ui) header: PageHeaderView,
}

impl ProfileFormPage {
    pub(in crate::app::ui) fn render() -> Self {
        let body = gtk4::Box::new(gtk4::Orientation::Vertical, 12);
        let name = libadwaita::EntryRow::builder()
            .title(ui_text::PROFILE_SETUP_NAME)
            .build();
        let name_group = libadwaita::PreferencesGroup::new();
        name_group.add(&name);

        let create_passkey = RadioButtonView::new(ui_text::PROFILE_SETUP_CREATE_PASSKEY);
        create_passkey.widget().set_active(true);
        let create_passkey_row = libadwaita::ActionRow::builder()
            .title(ui_text::PROFILE_SETUP_CREATE_PASSKEY)
            .subtitle(ui_text::PROFILE_SETUP_CREATE_PASSKEY_DESCRIPTION)
            .activatable_widget(create_passkey.widget())
            .build();
        create_passkey_row.add_prefix(create_passkey.widget());

        let existing_passkey = RadioButtonView::new(ui_text::PROFILE_SETUP_EXISTING_PASSKEY);
        existing_passkey
            .widget()
            .set_group(Some(create_passkey.widget()));
        let existing_passkey_row = libadwaita::ActionRow::builder()
            .title(ui_text::PROFILE_SETUP_EXISTING_PASSKEY)
            .subtitle(ui_text::PROFILE_SETUP_EXISTING_PASSKEY_DESCRIPTION)
            .activatable_widget(existing_passkey.widget())
            .build();
        existing_passkey_row.add_prefix(existing_passkey.widget());

        let passkey_group = libadwaita::PreferencesGroup::builder()
            .title(ui_text::PROFILE_SETUP_PASSKEY)
            .build();
        passkey_group.add(&create_passkey_row);
        passkey_group.add(&existing_passkey_row);

        let sk_format = RadioButtonView::new(ui_text::PROFILE_SETUP_SECURITY_KEY_FORMAT);
        let sk_row = libadwaita::ActionRow::builder()
            .title(ui_text::PROFILE_SETUP_SECURITY_KEY_FORMAT)
            .subtitle(ui_text::PROFILE_SETUP_SECURITY_KEY_DESCRIPTION)
            .activatable_widget(sk_format.widget())
            .build();
        sk_row.add_prefix(sk_format.widget());
        let ecdsa_format = RadioButtonView::new(ui_text::PROFILE_SETUP_ECDSA_FORMAT);
        ecdsa_format.widget().set_group(Some(sk_format.widget()));
        ecdsa_format.widget().set_active(true);
        let ecdsa_row = libadwaita::ActionRow::builder()
            .title(ui_text::PROFILE_SETUP_ECDSA_FORMAT)
            .subtitle(ui_text::PROFILE_SETUP_ECDSA_DESCRIPTION)
            .activatable_widget(ecdsa_format.widget())
            .build();
        ecdsa_row.add_prefix(ecdsa_format.widget());
        let format_group = libadwaita::PreferencesGroup::builder()
            .title(ui_text::PROFILE_SETUP_SIGNATURE_FORMAT)
            .description(ui_text::PROFILE_SETUP_SIGNATURE_DESCRIPTION)
            .build();
        format_group.add(&ecdsa_row);
        format_group.add(&sk_row);
        let advanced = gtk4::Expander::builder()
            .label(ui_text::PROFILE_SETUP_ADVANCED)
            .child(&format_group)
            .build();
        let status = gtk4::Label::builder()
            .xalign(0.0)
            .wrap(true)
            .visible(false)
            .build();
        status.add_css_class("error");

        let cancel = gtk4::Button::with_label(ui_text::CANCEL);
        let create = gtk4::Button::with_label(ui_text::PROFILE_SETUP_CREATE);
        create.add_css_class("suggested-action");
        let actions = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
        actions.append(&cancel);
        actions.append(&create);
        let action_bar = gtk4::ActionBar::new();
        action_bar.pack_end(&actions);

        body.append(&name_group);
        body.append(&passkey_group);
        body.append(&advanced);
        body.append(&status);
        let header = PageHeaderView::new();
        let wrapper = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        wrapper.append(&header.root);
        wrapper.append(&body);
        let root = libadwaita::ToolbarView::builder()
            .accessible_role(gtk4::AccessibleRole::Group)
            .build();
        root.upcast_ref::<gtk4::Widget>()
            .update_property(&[gtk4::accessible::Property::Label(
                ui_text::PROFILE_SETUP_FORM_ACCESSIBILITY,
            )]);
        root.set_content(Some(&wrapper));
        root.add_bottom_bar(&action_bar);
        Self {
            root: root.upcast(),
            name,
            create_passkey,
            existing_passkey,
            sk_format,
            ecdsa_format,
            advanced,
            status,
            cancel,
            create,
            header,
        }
    }
}
