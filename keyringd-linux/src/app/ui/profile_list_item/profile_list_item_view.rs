use keyringd_ui_core::profile_list_item::ProfileActions;
use keyringd_ui_core::profile_list_item::ProfileListItemEvent;
use keyringd_ui_core::profile_list_item::ProfileListItemProps;
use keyringd_ui_core::text as ui_text;

use keyringd_ui_core::runtime::UiEventTarget;

use gtk4::glib::object::CastNone;
use gtk4::glib::prelude::ToValue;
use gtk4::glib::types::StaticType;
use gtk4::prelude::{
    AccessibleExtManual, ActionableExt, BoxExt, ButtonExt, EditableExt, EntryExt, PopoverExt,
    WidgetExt,
};
use keyringd_core::model::{Profile, ProfileId};
use libadwaita::prelude::{ActionRowExt, PreferencesRowExt};

pub(crate) struct ProfileListItemView;

impl ProfileListItemView {
    pub(crate) fn build(
        props: ProfileListItemProps,
        profiles: &[Profile],
        on_event: UiEventTarget<ProfileListItemEvent>,
    ) -> libadwaita::ActionRow {
        let subtitle = props.subtitle();
        let row_actions = ProfileActions::for_profile(
            &props.profile,
            ui_text::DEVICE_KEY_PROTECTION_CHANGE_PASSWORD,
            (
                profiles
                    .iter()
                    .position(|p| p.id() == props.profile.id())
                    .expect("the rendered profile belongs to the list"),
                profiles.len(),
            ),
        );
        let profile = &props.profile;
        let editing_name = props.editing_name.as_deref();
        let row = libadwaita::ActionRow::builder()
            .title("")
            .subtitle("")
            .build();

        if let Some(editing_name) = editing_name {
            Self::add_name_editor(&row, profile, editing_name, &subtitle, on_event.clone());
        } else {
            Self::add_profile_summary(&row, profile, &subtitle, on_event.clone());
            Self::add_drag_reordering(&row, profile, profiles, on_event.clone());
        }

        let actions = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(0)
            .accessible_role(gtk4::AccessibleRole::Group)
            .build();
        actions.set_margin_top(6);
        actions.set_margin_bottom(6);
        actions.set_margin_start(6);
        actions.set_margin_end(6);
        for action in &row_actions.items {
            if action.separator_before {
                let separator = gtk4::Separator::new(gtk4::Orientation::Horizontal);
                separator.set_margin_top(6);
                separator.set_margin_bottom(6);
                actions.append(&separator);
            }
            let button = gtk4::Button::builder()
                .label(action.label)
                .halign(gtk4::Align::Fill)
                .sensitive(action.enabled)
                .build();
            Self::style_menu_button(&button);
            let action = action.clone();
            let on_event = on_event.clone();
            button.connect_clicked(move |button| {
                Self::close_menu(button);
                if let Some(event) = action.activate() {
                    on_event.send(event);
                }
            });
            actions.append(&button);
        }
        let popover = gtk4::Popover::builder().child(&actions).build();
        actions.update_property(&[gtk4::accessible::Property::Label(
            &keyringd_ui_core::profile_list_item::actions_menu_for(profile.display_name()),
        )]);
        let menu_icon = gtk4::Image::builder()
            .icon_name("open-menu-symbolic")
            .accessible_role(gtk4::AccessibleRole::Presentation)
            .build();
        let menu = gtk4::MenuButton::builder()
            .child(&menu_icon)
            .tooltip_text(ui_text::PROFILE_LIST_ITEM_ACTIONS)
            .popover(&popover)
            .valign(gtk4::Align::Center)
            .build();
        let menu_label = keyringd_ui_core::profile_list_item::actions_for(profile.display_name());
        menu.update_property(&[gtk4::accessible::Property::Label(&menu_label)]);
        Self::add_enabled_switch(&row, profile, row_actions, on_event);
        row.add_suffix(&menu);
        row
    }

    fn add_profile_summary(
        row: &libadwaita::ActionRow,
        profile: &Profile,
        subtitle: &str,
        on_event: UiEventTarget<ProfileListItemEvent>,
    ) {
        let title = gtk4::Label::builder()
            .label(profile.display_name())
            .xalign(0.0)
            .ellipsize(gtk4::pango::EllipsizeMode::End)
            .css_classes(["heading"])
            .tooltip_text(ui_text::PROFILE_LIST_ITEM_RENAME)
            .build();
        title.set_cursor_from_name(Some("pointer"));
        let rename = gtk4::GestureClick::new();
        let profile_id = profile.id().clone();
        let current_name = profile.display_name().to_owned();
        rename.connect_released(move |_, _, _, _| {
            on_event.send(ProfileListItemEvent::BeginRename {
                id: profile_id.clone(),
                name: current_name.clone(),
            });
        });
        title.add_controller(rename);

        let subtitle = gtk4::Label::builder()
            .label(subtitle)
            .xalign(0.0)
            .ellipsize(gtk4::pango::EllipsizeMode::End)
            .css_classes(["caption", "dim-label"])
            .build();
        let summary = gtk4::Box::new(gtk4::Orientation::Vertical, 2);
        summary.set_hexpand(true);
        summary.set_margin_top(8);
        summary.set_margin_bottom(8);
        summary.append(&title);
        summary.append(&subtitle);
        row.add_prefix(&summary);
    }

    fn add_drag_reordering(
        row: &libadwaita::ActionRow,
        profile: &Profile,
        profiles: &[Profile],
        on_event: UiEventTarget<ProfileListItemEvent>,
    ) {
        let handle = gtk4::Image::builder()
            .icon_name("list-drag-handle-symbolic")
            .tooltip_text(ui_text::PROFILE_LIST_ITEM_DRAG)
            .valign(gtk4::Align::Center)
            .build();
        let handle_label =
            keyringd_ui_core::profile_list_item::drag_profile(profile.display_name());
        handle.update_property(&[gtk4::accessible::Property::Label(&handle_label)]);
        handle.add_css_class("dim-label");

        let dragged_id = profile.id().as_str().to_owned();
        let drag_source = gtk4::DragSource::builder()
            .actions(gtk4::gdk::DragAction::MOVE)
            .build();
        drag_source.connect_prepare(move |_, _, _| {
            Some(gtk4::gdk::ContentProvider::for_value(
                &dragged_id.to_value(),
            ))
        });
        let row_for_drag_icon = row.clone();
        drag_source.connect_drag_begin(move |source, _| {
            let preview = gtk4::WidgetPaintable::new(Some(&row_for_drag_icon));
            source.set_icon(
                Some(&preview),
                24,
                row_for_drag_icon.height().saturating_div(2),
            );
        });
        handle.add_controller(drag_source);
        row.add_prefix(&handle);

        let original_ids: Vec<ProfileId> = profiles
            .iter()
            .map(|profile| profile.id().clone())
            .collect();
        let target_id = profile.id().clone();
        let row_for_drop = row.clone();
        let drop_target = gtk4::DropTarget::new(String::static_type(), gtk4::gdk::DragAction::MOVE);
        drop_target.connect_drop(move |_, value, _, y| {
            let Ok(dragged_id) = value.get::<String>() else {
                return false;
            };
            let Some(target) = original_ids.iter().position(|id| id == &target_id) else {
                return false;
            };

            let insert_after = y >= f64::from(row_for_drop.height()) / 2.0;
            let Some(reordered) = keyringd_ui_core::profile_list::reorder_profile(
                &original_ids,
                &dragged_id,
                target + usize::from(insert_after),
            ) else {
                return false;
            };
            if reordered == original_ids {
                return true;
            }
            on_event.send(ProfileListItemEvent::Reorder(reordered));
            true
        });
        row.add_controller(drop_target);
    }

    fn add_enabled_switch(
        row: &libadwaita::ActionRow,
        profile: &Profile,
        actions: ProfileActions,
        on_event: UiEventTarget<ProfileListItemEvent>,
    ) {
        let enabled = gtk4::Switch::builder()
            .active(profile.is_enabled())
            .valign(gtk4::Align::Center)
            .build();
        let enabled_label =
            keyringd_ui_core::profile_list_item::enable_profile(profile.display_name());
        enabled.update_property(&[gtk4::accessible::Property::Label(&enabled_label)]);
        enabled.set_action_name(None);
        enabled.connect_active_notify(move |enabled| {
            let requested = enabled.is_active();
            if let Some(event) = actions.enabled_changed(requested) {
                on_event.send(event);
            }
        });
        row.add_suffix(&enabled);
    }

    fn add_name_editor(
        row: &libadwaita::ActionRow,
        profile: &Profile,
        editing_name: &str,
        subtitle: &str,
        on_event: UiEventTarget<ProfileListItemEvent>,
    ) {
        row.set_title("");
        row.set_subtitle("");
        let editor = gtk4::Entry::builder()
            .text(editing_name)
            .hexpand(true)
            .build();
        let editor_label =
            keyringd_ui_core::profile_list_item::profile_name(profile.display_name());
        editor.update_property(&[gtk4::accessible::Property::Label(&editor_label)]);
        let profile_id = profile.id().clone();
        let on_changed = on_event.clone();
        let changed_id = profile_id.clone();
        editor.connect_changed(move |editor| {
            on_changed.send(ProfileListItemEvent::RenameNameChanged {
                id: changed_id.clone(),
                name: editor.text().to_string(),
            });
        });
        let on_activate = on_event.clone();
        let activate_id = profile_id.clone();
        editor.connect_activate(move |editor| {
            on_activate.send(ProfileListItemEvent::RenameNameChanged {
                id: activate_id.clone(),
                name: editor.text().to_string(),
            });
            on_activate.send(ProfileListItemEvent::RenameRequested(activate_id.clone()));
        });
        let cancel = gtk4::EventControllerKey::new();
        cancel.connect_key_pressed(move |_, key, _, _| {
            if key != gtk4::gdk::Key::Escape {
                return gtk4::glib::Propagation::Proceed;
            }
            on_event.send(ProfileListItemEvent::RenameCancelled(profile_id.clone()));
            gtk4::glib::Propagation::Stop
        });
        editor.add_controller(cancel);

        let subtitle = gtk4::Label::builder()
            .label(subtitle)
            .xalign(0.0)
            .ellipsize(gtk4::pango::EllipsizeMode::End)
            .css_classes(["caption", "dim-label"])
            .build();
        let content = gtk4::Box::new(gtk4::Orientation::Vertical, 3);
        content.set_hexpand(true);
        content.set_margin_top(8);
        content.set_margin_bottom(8);
        content.append(&editor);
        content.append(&subtitle);
        row.add_prefix(&content);
        editor.grab_focus();
        editor.select_region(0, -1);
    }

    fn style_menu_button(button: &gtk4::Button) {
        button.add_css_class("flat");
        if let Some(label) = button.child().and_downcast::<gtk4::Label>() {
            label.set_xalign(0.0);
        }
    }

    fn close_menu(button: &gtk4::Button) {
        if let Some(popover) = button
            .ancestor(gtk4::Popover::static_type())
            .and_downcast::<gtk4::Popover>()
        {
            popover.popdown();
        }
    }
}
