use gtk4::glib::object::Cast;
use gtk4::prelude::AccessibleExtManual;
use gtk4::prelude::{BoxExt, WidgetExt};

use keyringd_ui_core::text as ui_text;

use crate::app::ui::page_header::PageHeaderView;

pub(in crate::app::ui) struct CompletionPage {
    pub(in crate::app::ui) root: gtk4::Widget,
    pub(in crate::app::ui) open_trust_entries: gtk4::Button,
    pub(in crate::app::ui) done: gtk4::Button,
    pub(in crate::app::ui) header: PageHeaderView,
}

impl CompletionPage {
    pub(in crate::app::ui) fn render() -> Self {
        let open_trust_entries = gtk4::Button::with_label(ui_text::PROFILE_SETUP_PUBLIC_KEYS_LABEL);
        let done = gtk4::Button::with_label(ui_text::DONE);
        done.add_css_class("suggested-action");
        let action_bar = gtk4::ActionBar::new();
        action_bar.pack_end(&done);
        action_bar.pack_end(&open_trust_entries);
        let header = PageHeaderView::new();
        let wrapper = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        wrapper.append(&header.root);
        let root = libadwaita::ToolbarView::builder()
            .accessible_role(gtk4::AccessibleRole::Group)
            .build();
        root.upcast_ref::<gtk4::Widget>()
            .update_property(&[gtk4::accessible::Property::Label(
                ui_text::PROFILE_SETUP_COMPLETED_ACCESSIBILITY,
            )]);
        root.set_content(Some(&wrapper));
        root.add_bottom_bar(&action_bar);
        Self {
            root: root.upcast(),
            open_trust_entries,
            done,
            header,
        }
    }
}
