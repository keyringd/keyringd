use crate::system::application_resources::{APPLICATION_LICENSE, THIRD_PARTY_LICENSES};
use gtk4::prelude::*;
use keyringd_ui_core::about_window::LicenseDocument;
use keyringd_ui_core::text as ui_text;

pub(crate) struct LicenseWindow;
impl LicenseWindow {
    pub(crate) fn present(parent: &gtk4::Window, document: LicenseDocument) {
        let contents = match document {
            LicenseDocument::Application => APPLICATION_LICENSE,
            LicenseDocument::ThirdParty => THIRD_PARTY_LICENSES,
        };
        let buffer = gtk4::TextBuffer::builder().text(contents).build();
        let text = gtk4::TextView::builder()
            .buffer(&buffer)
            .editable(false)
            .cursor_visible(false)
            .wrap_mode(gtk4::WrapMode::WordChar)
            .left_margin(24)
            .right_margin(24)
            .top_margin(24)
            .bottom_margin(24)
            .build();
        text.update_property(&[gtk4::accessible::Property::Label(
            ui_text::ABOUT_LICENSE_ACCESSIBILITY,
        )]);
        let scroll = gtk4::ScrolledWindow::builder()
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .child(&text)
            .build();
        let toolbar = libadwaita::ToolbarView::new();
        toolbar.add_top_bar(&libadwaita::HeaderBar::new());
        toolbar.set_content(Some(&scroll));
        let window = libadwaita::Window::builder()
            .title(document.title())
            .transient_for(parent)
            .modal(true)
            .default_width(720)
            .default_height(560)
            .content(&toolbar)
            .build();
        let escape = gtk4::EventControllerKey::new();
        escape.set_propagation_phase(gtk4::PropagationPhase::Capture);
        let weak_window = window.downgrade();
        escape.connect_key_pressed(move |_, key, _, _| {
            if key == gtk4::gdk::Key::Escape {
                if let Some(window) = weak_window.upgrade() {
                    window.close();
                }
                gtk4::glib::Propagation::Stop
            } else {
                gtk4::glib::Propagation::Proceed
            }
        });
        window.add_controller(escape);
        window.present();
    }
}
