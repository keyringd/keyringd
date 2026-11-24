use gtk4::prelude::{BoxExt, WidgetExt};
use keyringd_ui_core::profile_setup_window::ProfileSetupEvent;

use keyringd_ui_core::runtime::UiEventTarget;
use keyringd_ui_core::text as ui_text;
use std::cell::Cell;
use std::rc::Rc;

use gtk4::glib::object::Cast;
use gtk4::prelude::CheckButtonExt;
use libadwaita::prelude::{ActionRowExt, PreferencesGroupExt};

use crate::app::ui::page_header::PageHeaderView;
use crate::app::ui::radio_button::RadioButtonView;
use keyringd_ui_core::profile_setup_window::PasskeyNotFoundChoice;

pub(in crate::app::ui) struct PasskeyNotFoundPage {
    pub root: gtk4::Widget,
    pub cancel: gtk4::Button,
    pub continue_button: gtk4::Button,
    pub(in crate::app::ui) header: PageHeaderView,
    try_another: RadioButtonView,
    create_new: RadioButtonView,
    rendering: Rc<Cell<bool>>,
}

impl PasskeyNotFoundPage {
    pub fn render(events: UiEventTarget<ProfileSetupEvent>) -> Self {
        let try_another = RadioButtonView::new(ui_text::PROFILE_SETUP_TRY_ANOTHER_DEVICE);
        try_another.widget().set_active(true);
        let retry_row = libadwaita::ActionRow::builder()
            .title(ui_text::PROFILE_SETUP_TRY_ANOTHER_DEVICE)
            .subtitle(ui_text::PROFILE_SETUP_TRY_ANOTHER_DESCRIPTION)
            .activatable_widget(try_another.widget())
            .build();
        retry_row.add_prefix(try_another.widget());

        let create_new = RadioButtonView::new(ui_text::PROFILE_SETUP_CREATE_INSTEAD);
        create_new.widget().set_group(Some(try_another.widget()));
        let create_row = libadwaita::ActionRow::builder()
            .title(ui_text::PROFILE_SETUP_CREATE_INSTEAD)
            .subtitle(ui_text::PROFILE_SETUP_CREATE_INSTEAD_DESCRIPTION)
            .activatable_widget(create_new.widget())
            .build();
        create_row.add_prefix(create_new.widget());

        let group = libadwaita::PreferencesGroup::new();
        group.add(&retry_row);
        group.add(&create_row);
        let body = gtk4::Box::new(gtk4::Orientation::Vertical, 12);
        body.append(&group);
        let cancel = gtk4::Button::with_label(ui_text::CANCEL);
        let continue_button = gtk4::Button::with_label(ui_text::CONTINUE);
        continue_button.add_css_class("suggested-action");
        let actions = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
        actions.append(&cancel);
        actions.append(&continue_button);
        let bar = gtk4::ActionBar::new();
        bar.pack_end(&actions);
        let header = PageHeaderView::new();
        let wrapper = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        wrapper.append(&header.root);
        wrapper.append(&body);
        let root = libadwaita::ToolbarView::new();
        root.set_content(Some(&wrapper));
        root.add_bottom_bar(&bar);

        let rendering = Rc::new(Cell::new(false));
        for (button, choice) in [
            (try_another.clone(), PasskeyNotFoundChoice::TryAnotherDevice),
            (create_new.clone(), PasskeyNotFoundChoice::CreateNewPasskey),
        ] {
            let events = events.clone();
            let rendering = Rc::clone(&rendering);
            button.widget().connect_toggled(move |button| {
                if button.is_active() && !rendering.get() {
                    events.send(ProfileSetupEvent::PasskeyNotFoundChoiceChanged(choice));
                }
            });
        }

        Self {
            root: root.upcast(),
            cancel,
            continue_button,
            header,
            try_another,
            create_new,
            rendering,
        }
    }

    pub fn render_choice(&self, choice: PasskeyNotFoundChoice) {
        self.rendering.set(true);
        match choice {
            PasskeyNotFoundChoice::TryAnotherDevice => self.try_another.widget().set_active(true),
            PasskeyNotFoundChoice::CreateNewPasskey => self.create_new.widget().set_active(true),
        }
        self.rendering.set(false);
    }
}
