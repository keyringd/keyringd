use keyringd_ui_core::text as ui_text;

use keyringd_ui_core::profile_list_item::ProfileListItemProps;
use keyringd_ui_core::runtime::UiEventTarget;
use windows_reactor::{
    AccessibilityExt, Element, GridChildExt, GridLength, HorizontalAlignment, Icon, Image,
    LayoutExt, PaddingExt, TextStyleExt, ThemeRef, Thickness, VerticalAlignment, border, button,
    grid, text_block, vstack,
};

use crate::app::ui::profile_list_item::ProfileListItemView;
use keyringd_ui_core::profile_list::ProfileListProps;
use windows_reactor::{ElementRef, TextBoxHandle};

use keyringd_ui_core::profile_list::ProfileListEvent;

use windows_reactor::{SelectionMode, list_view};

pub(crate) struct ProfileListView;

impl ProfileListView {
    pub(crate) fn render(
        props: ProfileListProps,
        edit_name_ref: ElementRef<TextBoxHandle>,
        on_event: UiEventTarget<ProfileListEvent>,
        key_icon_uri: &str,
    ) -> Element {
        let saving = props.saving;
        let empty_add = on_event.clone();
        if props.profiles.is_empty() {
            return vstack((
                Image::new(key_icon_uri)
                    .width(112.0)
                    .height(112.0)
                    .automation_name(ui_text::PROFILE_LIST_KEY_ACCESSIBILITY)
                    .horizontal_alignment(HorizontalAlignment::Center),
                vstack((
                    text_block(ui_text::PROFILE_LIST_EMPTY_TITLE)
                        .automation_id("profiles-empty")
                        .font_size(24.0)
                        .semibold()
                        .horizontal_alignment(HorizontalAlignment::Center),
                    text_block(ui_text::PROFILE_LIST_EMPTY_DESCRIPTION)
                        .foreground(ThemeRef::SecondaryText)
                        .wrap()
                        .horizontal_alignment(HorizontalAlignment::Center),
                ))
                .spacing(12.0),
                button(ui_text::PROFILE_LIST_ADD)
                    .enabled(props.creation_enabled)
                    .automation_id("profiles-create")
                    .icon(Icon::font("\u{E710}"))
                    .accent()
                    .min_height(40.0)
                    .padding(Thickness::xy(20.0, 8.0))
                    .horizontal_alignment(HorizontalAlignment::Center)
                    .on_click(move || empty_add.send(ProfileListEvent::AddRequested)),
            ))
            .spacing(24.0)
            .max_width(540.0)
            .padding(Thickness::uniform(16.0))
            .horizontal_alignment(HorizontalAlignment::Center)
            .vertical_alignment(VerticalAlignment::Center)
            .into();
        }

        let add = on_event.clone();
        let header_action: Element = {
            button(ui_text::PROFILE_LIST_ADD)
                .enabled(props.creation_enabled)
                .automation_id("profiles-create")
                .accent()
                .on_click(move || add.send(ProfileListEvent::AddRequested))
                .horizontal_alignment(HorizontalAlignment::Right)
                .grid_column(1)
                .into()
        };

        let header: Element = {
            grid((
                vstack((
                    text_block(ui_text::PROFILE_LIST_TITLE)
                        .font_size(28.0)
                        .bold(),
                    text_block(ui_text::PROFILE_LIST_DESCRIPTION)
                        .foreground(ThemeRef::SecondaryText)
                        .wrap(),
                ))
                .spacing(4.0)
                .grid_column(0),
                header_action,
            ))
            .columns([GridLength::STAR, GridLength::Auto])
            .into()
        };

        let count = props.profiles.len();
        let item_event = on_event.map(|event| ProfileListEvent::ProfileAction(Box::new(event)));
        let reorder_event = on_event;
        let reorder_source = props.profiles.clone();
        let editing = props.editing;
        let list: Element = list_view(props.profiles, move |profile, index| {
            ProfileListItemView::render(
                ProfileListItemProps {
                    saving,
                    profile: profile.clone(),
                    editing_name: editing
                        .as_ref()
                        .filter(|(id, _)| id == profile.id())
                        .map(|(_, name)| name.clone()),
                },
                (index, count),
                edit_name_ref.clone(),
                item_event.clone(),
            )
        })
        .with_key_selector(|profile| profile.id().as_str().to_string())
        .selection_mode(SelectionMode::None)
        .can_drag_items(!saving)
        .can_reorder_items(!saving)
        .allow_drop(!saving)
        .on_reorder(move |permutation: Vec<usize>| {
            let ids = permutation
                .iter()
                .filter_map(|index| reorder_source.get(*index))
                .map(|profile| profile.id().clone())
                .collect();
            reorder_event.send(ProfileListEvent::ProfileAction(Box::new(
                keyringd_ui_core::profile_list_item::ProfileListItemEvent::Reorder(ids),
            )));
        })
        .into();
        grid((border(header).grid_row(0), border(list).grid_row(1)))
            .rows([GridLength::Auto, GridLength::STAR])
            .row_spacing(12.0)
            .into()
    }
}
