use crate::app::APPLICATION_ID;
use crate::app::ui::license_window::LicenseWindow;
use gtk4::prelude::*;
use keyringd_ui_core::about_window::LicenseDocument;
use keyringd_ui_core::text as ui_text;

pub(crate) struct AboutWindow;
impl AboutWindow {
    pub(crate) fn present(parent: &gtk4::Window) {
        let content = gtk4::Box::new(gtk4::Orientation::Vertical, 12);
        content.set_margin_top(24);
        content.set_margin_bottom(24);
        content.set_margin_start(24);
        content.set_margin_end(24);
        content.append(
            &gtk4::Image::builder()
                .icon_name(APPLICATION_ID)
                .pixel_size(64)
                .build(),
        );
        let name = gtk4::Label::new(Some("Keyringd"));
        name.add_css_class("title-1");
        content.append(&name);
        for text in [
            keyringd_ui_core::about_window::version(env!("CARGO_PKG_VERSION")),
            ui_text::ABOUT_MAINTAINERS.into(),
            ui_text::ABOUT_LICENSE_NAME.into(),
        ] {
            content.append(&gtk4::Label::new(Some(&text)));
        }
        for (uri, label) in [
            (ui_text::ABOUT_WEBSITE_URL, ui_text::ABOUT_WEBSITE),
            (ui_text::ABOUT_ISSUES_URL, ui_text::ABOUT_REPORT_ISSUE),
        ] {
            content.append(&gtk4::LinkButton::with_label(uri, label));
        }
        let toolbar = libadwaita::ToolbarView::new();
        toolbar.add_top_bar(&libadwaita::HeaderBar::new());
        toolbar.set_content(Some(&content));
        let about = libadwaita::Window::builder()
            .title(ui_text::ABOUT_TITLE)
            .transient_for(parent)
            .modal(true)
            .resizable(false)
            .default_width(420)
            .content(&toolbar)
            .build();
        for (document, label) in [
            (LicenseDocument::Application, ui_text::ABOUT_LICENSE),
            (
                LicenseDocument::ThirdParty,
                ui_text::ABOUT_THIRD_PARTY_LICENSES,
            ),
        ] {
            let button = gtk4::Button::with_label(label);
            let weak_about = about.downgrade();
            button.connect_clicked(move |_| {
                if let Some(about) = weak_about.upgrade() {
                    LicenseWindow::present(about.upcast_ref(), document);
                }
            });
            content.append(&button);
        }
        about.present();
    }
}
