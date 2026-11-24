use gtk4::prelude::{BoxExt, WidgetExt};
use keyringd_core::model::passkey::CredentialId;
use keyringd_ui_core::profile_setup_window::ProfileSetupEvent;

use keyringd_ui_core::runtime::UiEventTarget;
use keyringd_ui_core::text as ui_text;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

use gtk4::glib::object::Cast;
use gtk4::prelude::{AccessibleExtManual, CheckButtonExt};
use libadwaita::prelude::{ActionRowExt, PreferencesGroupExt};

use crate::app::ui::page_header::PageHeaderView;
use crate::app::ui::radio_button::RadioButtonView;
use keyringd_ui_core::profile_setup_window::PasskeyCredentialSelection;

pub(in crate::app::ui) struct PasskeySelectionPage {
    root: gtk4::Widget,
    group: libadwaita::PreferencesGroup,
    rows: RefCell<Vec<(RadioButtonView, libadwaita::ActionRow, CredentialId)>>,
    events: UiEventTarget<ProfileSetupEvent>,
    rendering: Rc<Cell<bool>>,
    pub(in crate::app::ui) cancel: gtk4::Button,
    pub(in crate::app::ui) continue_button: gtk4::Button,
    pub(in crate::app::ui) header: PageHeaderView,
}

impl PasskeySelectionPage {
    pub(in crate::app::ui) fn new(events: UiEventTarget<ProfileSetupEvent>) -> Self {
        let group = libadwaita::PreferencesGroup::new();
        let body = gtk4::Box::new(gtk4::Orientation::Vertical, 12);
        body.append(&group);
        let cancel = gtk4::Button::with_label(ui_text::CANCEL);
        let continue_button = gtk4::Button::with_label(ui_text::CONTINUE);
        continue_button.add_css_class("suggested-action");
        let actions = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
        actions.append(&cancel);
        actions.append(&continue_button);
        let action_bar = gtk4::ActionBar::new();
        action_bar.pack_end(&actions);
        let header = PageHeaderView::new();
        let wrapper = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        wrapper.append(&header.root);
        wrapper.append(&body);
        let root = libadwaita::ToolbarView::builder()
            .accessible_role(gtk4::AccessibleRole::Group)
            .build();
        root.upcast_ref::<gtk4::Widget>()
            .update_property(&[gtk4::accessible::Property::Label(
                ui_text::PROFILE_SETUP_PASSKEY_CHOICES_ACCESSIBILITY,
            )]);
        root.set_content(Some(&wrapper));
        root.add_bottom_bar(&action_bar);
        Self {
            root: root.upcast(),
            group,
            rows: RefCell::new(Vec::new()),
            events,
            rendering: Rc::new(Cell::new(false)),
            cancel,
            continue_button,
            header,
        }
    }

    pub(in crate::app::ui) fn widget(&self) -> &gtk4::Widget {
        &self.root
    }

    pub(in crate::app::ui) fn render(&self, selection: &PasskeyCredentialSelection) {
        // GTK emits toggled synchronously for programmatic selection changes.
        // Rendering must not dispatch another event while the model is borrowed.
        self.rendering.set(true);
        self.render_selection(selection);
        self.rendering.set(false);
    }

    fn render_selection(&self, selection: &PasskeyCredentialSelection) {
        let matches_existing = {
            let rows = self.rows.borrow();
            rows.len() == selection.candidates().len()
                && rows
                    .iter()
                    .zip(selection.candidates())
                    .all(|((_, _, id), candidate)| id == candidate.credential_id())
        };
        if matches_existing {
            if let Some((choice, _, _)) = self.rows.borrow().get(selection.selected_index())
                && !choice.widget().is_active()
            {
                choice.widget().set_active(true);
            }
            return;
        }
        for (_, row, _) in self.rows.borrow_mut().drain(..) {
            self.group.remove(&row);
        }
        let mut rows = self.rows.borrow_mut();
        for (index, candidate) in selection.candidates().iter().enumerate() {
            let choice = RadioButtonView::new(candidate.label());
            if let Some((first, _, _)) = rows.first() {
                choice.widget().set_group(Some(first.widget()));
            }
            choice
                .widget()
                .set_active(index == selection.selected_index());
            let events = self.events.clone();
            let rendering = Rc::clone(&self.rendering);
            choice.widget().connect_toggled(move |choice| {
                if choice.is_active() && !rendering.get() {
                    events.send(ProfileSetupEvent::CredentialChoiceChanged(index));
                }
            });
            let row = libadwaita::ActionRow::builder()
                .title(candidate.label())
                .activatable_widget(choice.widget())
                .build();
            row.add_prefix(choice.widget());
            self.group.add(&row);
            rows.push((choice, row, candidate.credential_id().clone()));
        }
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::rc::Rc;
    use std::sync::Arc;

    use gtk4::prelude::{CheckButtonExt, WidgetExt};
    use keyringd_core::application::PasskeyChoice;
    use keyringd_core::application::PasskeyCredentialCandidate;
    use keyringd_core::application::ProfileSetupRequest;
    use keyringd_core::application::{ApplicationError, KeyringdApplication};
    use keyringd_core::test_support::{
        MemoryProfileRepository, SoftwareDeviceKey, TestPasskeyService,
    };

    use crate::app::ui::ComponentRuntime;
    use crate::app::ui::profile_setup_wizard::PasskeySelectionPage;
    use crate::app::ui::profile_setup_wizard::test_support::SelectionState;
    use keyringd_ui_core::profile_setup_window::{PasskeyCredentialSelection, ProfileSetupEvent};
    use keyringd_ui_core::runtime::UiEventTarget;

    fn credential_candidates() -> Vec<PasskeyCredentialCandidate> {
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            let application = KeyringdApplication::new(
                SoftwareDeviceKey::generating(),
                TestPasskeyService::new(),
                Arc::new(MemoryProfileRepository::default()),
            )
            .unwrap();
            for name in ["First", "Second"] {
                application
                    .setup_profile(
                        &Default::default(),
                        &Default::default(),
                        ProfileSetupRequest::new(name.into(), PasskeyChoice::CreateNew),
                        |_| {},
                        keyringd_core::test_support::reject_credential_selection,
                    )
                    .await
                    .unwrap();
            }
            let mut candidates = Vec::new();
            let _ = application
                .setup_profile(
                    &Default::default(),
                    &Default::default(),
                    ProfileSetupRequest::new("Recovered".into(), PasskeyChoice::Existing),
                    |_| {},
                    |choices| {
                        candidates = choices;
                        async { Err(ApplicationError::OperationCanceled) }
                    },
                )
                .await;
            assert_eq!(candidates.len(), 2);
            candidates
        })
    }

    keyringd_testkit::scenario! {
        group "PasskeySelectionPage::render" {
            case "Retry resets selection without dispatching during render" {
                verify "clear the old selection when retry renders without issuing a command" {
                    let candidates = credential_candidates();
                    gtk4::test_synced(move || {
                        libadwaita::init().unwrap();
                        let runtime = Rc::new(ComponentRuntime::new(SelectionState::default()));
                        let selection_events = Rc::new(Cell::new(0));
                        let runtime_for_choice = Rc::clone(&runtime);
                        let events_for_choice = Rc::clone(&selection_events);
                        let page = PasskeySelectionPage::new(UiEventTarget::new(move |event| {
                            let ProfileSetupEvent::CredentialChoiceChanged(index) = event else {
                                panic!("unexpected event");
                            };
                            events_for_choice.set(events_for_choice.get() + 1);
                            runtime_for_choice.dispatch(index);
                        }));
                        let selection = PasskeyCredentialSelection::new(candidates);
                        runtime.dispatch_and_render(0, |_| page.render(&selection));
                        assert_eq!(selection_events.get(), 0);

                        // User input must still reach the reducer.
                        let second = page.rows.borrow()[1].0.clone();
                        second
                            .widget()
                            .activate_action("radio.select", None)
                            .unwrap();
                        assert_eq!(selection_events.get(), 1);
                        assert_eq!(runtime.state().selected_index, 1);

                        // Reuse the same GTK rows when a retry starts with the default selection.
                        runtime.dispatch_and_render(0, |_| page.render(&selection));
                        assert!(page.rows.borrow()[0].0.widget().is_active());
                        assert!(!second.widget().is_active());
                        assert_eq!(selection_events.get(), 1);

                        // Rendering leaves user input enabled after resetting the selection.
                        second
                            .widget()
                            .activate_action("radio.select", None)
                            .unwrap();
                        assert_eq!(selection_events.get(), 2);
                        assert_eq!(runtime.state().selected_index, 1);
                    });
                }
            }
        }
    }
}
