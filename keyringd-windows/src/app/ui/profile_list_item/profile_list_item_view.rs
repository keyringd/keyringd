use keyringd_ui_core::text as ui_text;

use keyringd_ui_core::profile_list_item::ProfileActions;

use keyringd_ui_core::runtime::UiEventTarget;

use windows_reactor::{
    AccessibilityExt, BackgroundExt, Element, ElementRefExt, GridChildExt, GridLength,
    HorizontalAlignment, InputExt, KeyExt, KeyboardAccelerator, LayoutExt, MenuItemDef, PaddingExt,
    TextStyleExt, ThemeRef, Thickness, ToggleSwitch, TooltipExt, VerticalAlignment, VirtualKey,
    VirtualKeyModifiers, border, drop_down_button, grid, hstack, menu_item, menu_item_disabled,
    text_block, text_box, vstack,
};

use keyringd_ui_core::profile_list_item::ProfileListItemProps;

use keyringd_ui_core::profile_list_item::ProfileListItemEvent;
use windows_reactor::{ElementRef, TextBoxHandle};

pub(crate) struct ProfileListItemView;

impl ProfileListItemView {
    pub(crate) fn render(
        props: ProfileListItemProps,
        position: (usize, usize),
        edit_name_ref: ElementRef<TextBoxHandle>,
        on_event: UiEventTarget<ProfileListItemEvent>,
    ) -> Element {
        let status = props.subtitle();
        let profile = props.profile;
        let actions = ProfileActions::for_profile(
            &profile,
            ui_text::DEVICE_KEY_PROTECTION_RECONFIGURE,
            position,
        );
        let id = profile.id().clone();
        let name = profile.display_name().to_owned();
        let name_editor: Element = if let Some(edit_name) = props.editing_name {
            let edit = on_event.clone();
            let save = on_event.clone();
            let cancel = on_event.clone();
            let save_id = id.clone();
            let edit_id = id.clone();
            let cancel_id = id.clone();
            text_box(edit_name)
                .enabled(!props.saving)
                .automation_id("profiles-name-editor")
                .on_text_changed(move |name| {
                    edit.send(ProfileListItemEvent::RenameNameChanged {
                        id: edit_id.clone(),
                        name,
                    })
                })
                .element_ref(&edit_name_ref)
                .keyboard_accelerator(KeyboardAccelerator::new(
                    VirtualKey::Enter,
                    VirtualKeyModifiers::None,
                    move || save.send(ProfileListItemEvent::RenameRequested(save_id.clone())),
                ))
                .keyboard_accelerator(KeyboardAccelerator::new(
                    VirtualKey::Escape,
                    VirtualKeyModifiers::None,
                    move || cancel.send(ProfileListItemEvent::RenameCancelled(cancel_id.clone())),
                ))
                .width(240.0)
                .into()
        } else {
            let event = on_event.clone();
            let edit_id = id.clone();
            let edit_name = name.clone();
            text_block(name.clone())
                .semibold()
                .on_tapped(move || {
                    event.send(ProfileListItemEvent::BeginRename {
                        id: edit_id.clone(),
                        name: edit_name.clone(),
                    })
                })
                .into()
        };
        let mut menu = Vec::new();
        let mut menu_key = format!("profile-actions:{id:?}:{name:?}");
        for action in &actions.items {
            let label = action.label;
            if action.separator_before {
                menu.push(MenuItemDef::Separator);
            }
            menu.push(if action.enabled {
                menu_item(label)
            } else {
                menu_item_disabled(label)
            });
            menu_key.push_str(&format!(":{label}:{}", action.enabled));
        }
        let menu_actions = actions.items.clone();
        let menu_event = on_event.clone();
        // WinUI retains handlers on existing flyout items. Refresh the native menu
        // when its target or action availability changes, including recycled rows.
        let controls = drop_down_button("⋯")
            .enabled(!props.saving)
            .with_key(menu_key)
            .tooltip(ui_text::PROFILE_LIST_ITEM_ACTIONS)
            .automation_name(keyringd_ui_core::profile_list_item::actions_for(&name))
            .menu_flyout(menu)
            .on_item_clicked(move |label: String| {
                if let Some(event) = menu_actions
                    .iter()
                    .find(|action| action.label == label)
                    .and_then(|action| action.activate())
                {
                    menu_event.send(event);
                }
            })
            .grid_column(2);
        let enabled = profile.is_enabled();
        let enabled_name = profile.display_name().to_owned();
        let enabled_event = on_event;
        let enabled_toggle = ToggleSwitch::new(enabled)
            .on_content("")
            .off_content("")
            .enabled(!props.saving)
            .automation_name(keyringd_ui_core::profile_list_item::enable_profile(
                &enabled_name,
            ))
            .min_width(0.0)
            .on_toggled(move |toggled| {
                if let Some(event) = actions.enabled_changed(toggled) {
                    enabled_event.send(event);
                }
            })
            .vertical_alignment(VerticalAlignment::Center)
            .grid_column(1);
        let summary = hstack((
            text_block("⠿")
                .tooltip(ui_text::PROFILE_LIST_ITEM_DRAG)
                .font_size(20.0)
                .foreground(ThemeRef::SecondaryText),
            vstack((
                name_editor,
                text_block(status)
                    .font_size(12.0)
                    .foreground(ThemeRef::SecondaryText),
            ))
            .spacing(2.0),
        ))
        .spacing(12.0)
        .vertical_alignment(VerticalAlignment::Center)
        .horizontal_alignment(HorizontalAlignment::Left)
        .grid_column(0);
        border(
            grid((summary, enabled_toggle, controls))
                .columns([GridLength::STAR, GridLength::Auto, GridLength::Auto])
                .column_spacing(16.0),
        )
        .padding(Thickness::xy(16.0, 12.0))
        .margin(Thickness::xy(0.0, 4.0))
        .corner_radius(8.0)
        .background(ThemeRef::CardBackground)
        .border_brush(ThemeRef::CardStroke)
        .border_thickness(Thickness::uniform(1.0))
        .horizontal_alignment(HorizontalAlignment::Stretch)
        .into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    use windows_reactor::{Element, ElementRef, Reconciler};

    // Model WinUI's flyout lifetime: replacing the registered handler does not
    // change existing menu items; assigning MenuFlyoutItems builds new ones.

    use crate::app::ui::test_support::FlyoutBackend;

    fn profile(
        name: &str,
    ) -> (
        std::sync::Arc<
            keyringd_core::application::KeyringdApplication<
                keyringd_core::test_support::SoftwareDeviceKey,
                keyringd_core::test_support::TestPasskeyService,
            >,
        >,
        keyringd_core::model::Profile,
    ) {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                let app = keyringd_core::application::KeyringdApplication::new(
                    keyringd_core::test_support::SoftwareDeviceKey::new(7, b"row-test"),
                    keyringd_core::test_support::TestPasskeyService::new(),
                    std::sync::Arc::new(
                        keyringd_core::test_support::MemoryProfileRepository::default(),
                    ),
                )
                .unwrap();
                let profile = app
                    .setup_profile(
                        &Default::default(),
                        &Default::default(),
                        keyringd_core::application::ProfileSetupRequest::new(
                            name.to_owned(),
                            keyringd_core::application::PasskeyChoice::CreateNew,
                        ),
                        |_| {},
                        keyringd_core::test_support::reject_credential_selection,
                    )
                    .await
                    .unwrap();
                (app, profile)
            })
    }
    fn row(
        profile: keyringd_core::model::Profile,
        events: Rc<RefCell<Vec<ProfileListItemEvent>>>,
    ) -> Element {
        ProfileListItemView::render(
            ProfileListItemProps {
                saving: false,
                profile,
                editing_name: None,
            },
            (0, 1),
            ElementRef::default(),
            UiEventTarget::new(move |event| events.borrow_mut().push(event)),
        )
    }

    keyringd_testkit::scenario! {
        group "ProfileListItemView::render" {
            case "Target the Profile currently displayed in ROW for continuous deletion" {
                given "Prepare a profile list to reuse the same Native Row Slot" {
                    let events = Rc::new(RefCell::new(Vec::new()));
                    let mut reconciler = Reconciler::new(FlyoutBackend::default());
                    let mut previous = None;
                    let mut native = None;
                }
                when "Update Rows in order from the top and run Remove Menu" {
                    let mut removed = Vec::new();
                    let mut expected = Vec::new();
                    let mut menu_counts = Vec::new();
                    // ListView refreshes rows positionally when the item count decreases.
                    // Keep updating the same native slot as the preceding profile is removed.
                    for id in ["first", "second", "third"] {
                        let (_, profile) = profile(id);
                        expected.push(profile.id().as_str().to_owned());
                        let current = row(profile, events.clone());
                        native = reconciler.reconcile(previous.as_ref(), &current, native, Rc::new(|| {}));
                        menu_counts.push(reconciler.backend.menus.len());
                        reconciler
                            .backend
                            .menus
                            .values()
                            .next()
                            .unwrap()
                            .invoke_string(ui_text::REMOVE_PROFILE_MENU_ACTION.into());
                        let event = events.borrow_mut().pop().unwrap();
                        let ProfileListItemEvent::Remove { id: target, .. } = event else {
                            panic!("Remove event was not emitted");
                        };
                        removed.push(target.as_str().to_owned());
                        previous = Some(current);
                    }
                }
                then "Remove Menu does not overlap with ROW reuse" {
                    assert_eq!(menu_counts, [1, 1, 1]);
                }
                then "Deleting the Profile displayed in ROW at each operation point" {
                    assert_eq!(removed, expected);
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "ProfileListItemView::render" {
            case "Rename Menu uses updated profile name" {
                given "Prepare a List item built under the old Profile name and an updated Props" {
                    let events = Rc::new(RefCell::new(Vec::new()));
                    let mut reconciler = Reconciler::new(FlyoutBackend::default());
                    let (application, old_profile) = profile("Old name");
                    let profile_id = old_profile.id().clone();
                    let old = row(old_profile, events.clone());
                    let native = reconciler.reconcile(None, &old, None, Rc::new(|| {}));
                }
                when "Call ROW" {
                    application
                        .rename_profile(&profile_id, "New name".into())
                        .unwrap();
                    let new = row(application.profile(&profile_id).unwrap(), events.clone());
                    reconciler.reconcile(Some(&old), &new, native, Rc::new(|| {}));
                }
                then "Verify that the results match ProfileListItemEvent::BeginRename" {
                    assert_eq!(reconciler.backend.menus.len(), 1);
                    reconciler
                        .backend
                        .menus
                        .values()
                        .next()
                        .unwrap()
                        .invoke_string(ui_text::PROFILE_LIST_RENAME.into());
                    let event = events.borrow_mut().pop().unwrap();
                    assert!(
                        matches!(event, ProfileListItemEvent::BeginRename { id, name }
                                    if id == profile_id && name == "New name")
                    );
                }
            }
        }
    }
}
