use keyringd_ui_core::profile_list::{PROFILES_EMPTY_KEY_ICON, ProfileListEvent};
use keyringd_ui_core::text as ui_text;

use keyringd_ui_core::runtime::UiEventTarget;

use std::cell::RefCell;

use gtk4::prelude::{AccessibleExtManual, ButtonExt, WidgetExt};
use keyringd_core::model::Profile;
use keyringd_core::model::ProfileId;
use libadwaita::prelude::{PreferencesGroupExt, PreferencesPageExt};

use crate::app::ui::profile_list_item::ProfileListItemView;
use keyringd_ui_core::profile_list_item::ProfileListItemProps;

pub(crate) struct ProfileListView {
    root: gtk4::Stack,
    add: gtk4::Button,
    empty_add: gtk4::Button,
    group: libadwaita::PreferencesGroup,
    rows: RefCell<Vec<libadwaita::ActionRow>>,
    on_event: UiEventTarget<ProfileListEvent>,
}

impl ProfileListView {
    pub(crate) fn new(on_event: UiEventTarget<ProfileListEvent>) -> Self {
        let add = gtk4::Button::builder()
            .child(
                &libadwaita::ButtonContent::builder()
                    .icon_name("list-add-symbolic")
                    .label(ui_text::PROFILE_LIST_ADD)
                    .build(),
            )
            .tooltip_text(ui_text::PROFILE_LIST_ADD_TOOLTIP)
            .valign(gtk4::Align::Center)
            .build();
        let group = libadwaita::PreferencesGroup::builder()
            .title(ui_text::PROFILE_LIST_TITLE)
            .description(ui_text::PROFILE_LIST_DESCRIPTION)
            .header_suffix(&add)
            .build();
        let page = libadwaita::PreferencesPage::new();
        page.add(&group);

        let empty_add = gtk4::Button::builder()
            .child(
                &libadwaita::ButtonContent::builder()
                    .icon_name("list-add-symbolic")
                    .label(ui_text::PROFILE_LIST_ADD)
                    .build(),
            )
            .halign(gtk4::Align::Center)
            .css_classes(["suggested-action", "pill"])
            .build();
        let key_icon = gtk4::gdk::Texture::from_bytes(&gtk4::glib::Bytes::from_static(
            PROFILES_EMPTY_KEY_ICON,
        ))
        .expect("the embedded key icon is a valid PNG");
        let empty = libadwaita::StatusPage::builder()
            .paintable(&key_icon)
            .title(ui_text::PROFILE_LIST_EMPTY_TITLE)
            .description(ui_text::PROFILE_LIST_EMPTY_DESCRIPTION)
            .child(&empty_add)
            .build();
        let root = gtk4::Stack::builder()
            .accessible_role(gtk4::AccessibleRole::Group)
            .transition_type(gtk4::StackTransitionType::Crossfade)
            .build();
        root.update_property(&[gtk4::accessible::Property::Label(
            ui_text::PROFILE_LIST_TITLE,
        )]);
        root.add_named(&page, Some("profiles"));
        root.add_named(&empty, Some("empty"));

        let on_event = on_event;
        let on_add = on_event.clone();
        add.connect_clicked(move |_| on_add.send(ProfileListEvent::AddRequested));
        let on_empty_add = on_event.clone();
        empty_add.connect_clicked(move |_| on_empty_add.send(ProfileListEvent::AddRequested));

        Self {
            root,
            add,
            empty_add,
            group,
            rows: RefCell::new(Vec::new()),
            on_event,
        }
    }

    pub(crate) fn widget(&self) -> &gtk4::Stack {
        &self.root
    }

    pub(crate) fn render(
        &self,
        profiles: &[Profile],
        editing: Option<(&ProfileId, &str)>,
        saving: bool,
        creation_enabled: bool,
    ) {
        self.root.set_sensitive(!saving);
        self.add.set_sensitive(creation_enabled);
        self.empty_add.set_sensitive(creation_enabled);
        for row in self.rows.borrow_mut().drain(..) {
            self.group.remove(&row);
        }
        self.root.set_visible_child_name(if profiles.is_empty() {
            "empty"
        } else {
            "profiles"
        });
        let mut rows = self.rows.borrow_mut();
        for profile in profiles {
            let on_event = self
                .on_event
                .map(|event| ProfileListEvent::ProfileAction(Box::new(event)));
            let row = ProfileListItemView::build(
                ProfileListItemProps {
                    saving,
                    profile: profile.clone(),
                    editing_name: editing
                        .filter(|(editing_id, _)| *editing_id == profile.id())
                        .map(|(_, name)| name.to_owned()),
                },
                profiles,
                on_event,
            );
            self.group.add(&row);
            rows.push(row);
        }
    }
}
