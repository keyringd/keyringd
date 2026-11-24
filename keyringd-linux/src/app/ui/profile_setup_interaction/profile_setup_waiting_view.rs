use gtk4::glib::object::Cast;
use gtk4::prelude::{BoxExt, WidgetExt};
use keyringd_ui_core::text as ui_text;

use crate::app::ui::page_header::PageHeaderView;
use crate::app::ui::profile_setup_interaction::interaction_body;
use keyringd_ui_core::profile_setup_window::ProfileSetupInteractionPresentation;

pub(super) struct ProfileSetupWaitingView {
    widget: gtk4::Widget,
    pub(super) cancel: gtk4::Button,
    pub(super) header: PageHeaderView,
    heading: gtk4::Label,
    instruction: gtk4::Label,
    spinner: gtk4::Spinner,
}

impl ProfileSetupWaitingView {
    pub(super) fn new() -> Self {
        let body = interaction_body();
        let heading = gtk4::Label::builder().xalign(0.0).wrap(true).build();
        heading.add_css_class("heading");
        let instruction = gtk4::Label::builder().xalign(0.0).wrap(true).build();
        let spinner = gtk4::Spinner::new();
        spinner.set_halign(gtk4::Align::Center);
        spinner.set_margin_top(60);
        spinner.set_margin_bottom(60);
        let cancel = gtk4::Button::with_label(ui_text::CANCEL);
        let actions = gtk4::ActionBar::new();
        actions.pack_end(&cancel);
        body.append(&heading);
        body.append(&instruction);
        body.append(&spinner);
        let page_header = PageHeaderView::new();
        let wrapper = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        wrapper.append(&page_header.root);
        wrapper.append(&body);
        let page = libadwaita::ToolbarView::new();
        page.set_content(Some(&wrapper));
        page.add_bottom_bar(&actions);
        Self {
            widget: page.upcast(),
            cancel,
            header: page_header,
            heading,
            instruction,
            spinner,
        }
    }

    pub(super) fn widget(&self) -> &gtk4::Widget {
        &self.widget
    }

    pub(super) fn render(&self, presentation: &ProfileSetupInteractionPresentation) {
        self.heading.set_label(&presentation.title());
        self.instruction.set_label(&presentation.description());
        self.spinner.start();
    }

    pub(super) fn stop(&self) {
        self.spinner.stop();
    }
}
